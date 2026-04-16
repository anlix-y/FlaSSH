use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use crate::models::Server;
use std::sync::mpsc::Sender;
use colored::*;
use termion::raw::IntoRawMode;
use termion::terminal_size;

pub fn execute(server: &Server, command: &str, color: &str) {
    let tcp = TcpStream::connect(format!("{}:{}", server.host, server.port))
        .expect("Failed to connect");

    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();

    if let Some(key) = &server.key_path {
        sess.userauth_pubkey_file(
            &server.user,
            None,
            std::path::Path::new(key),
            None,
        ).expect("Key auth failed");

    } else if let Some(password) = &server.password {
        sess.userauth_password(&server.user, password)
            .expect("Password auth failed");

    } else {
        sess.userauth_agent(&server.user)
            .expect("SSH agent auth failed");
    }

    if !sess.authenticated() {
        panic!("Authentication failed");
    }

    let mut channel = sess.channel_session().unwrap();
    channel.exec(command).unwrap();

    let mut s = String::new();
    channel.read_to_string(&mut s).unwrap();

    println!("{}:", format!("{}@{}", server.user, server.host).color(color));
    println!("{}", s);

    channel.wait_close().unwrap();
}

pub fn interactive(server: &Server, color: &str) {
    let tcp = TcpStream::connect(format!("{}:{}", server.host, server.port))
        .expect("Failed to connect");

    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();

    if let Some(key) = &server.key_path {
        sess.userauth_pubkey_file(&server.user, None, Path::new(key), None)
            .expect("Key auth failed");
    } else if let Some(password) = &server.password {
        sess.userauth_password(&server.user, password)
            .expect("Password auth failed");
    } else {
        sess.userauth_agent(&server.user)
            .expect("SSH agent auth failed");
    }

    if !sess.authenticated() {
        panic!("Authentication failed");
    }

    println!("{} {}", "Connected to:".color(color), server.name.color(color).bold());

    let mut channel = sess.channel_session().unwrap();
    let (w, h) = terminal_size().unwrap_or((80, 24));
    channel.request_pty("xterm-256color", None, Some((w as u32, h as u32, 0, 0))).unwrap();
    channel.shell().unwrap();

    let mut channel_in = channel.stream(0);
    let mut channel_out = channel.stream(0);

    let mut stdout = std::io::stdout().into_raw_mode().unwrap();

    sess.set_blocking(false);

    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match channel_out.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = stdout.write_all(&buffer[..n]);
                    let _ = stdout.flush();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
        // When output stops, we should probably exit or signal it.
        // For a simple CLI, we can just exit the process if it's the only thing running.
        // But better to return from the function.
        std::process::exit(0);
    });

    let mut stdin = std::io::stdin();
    let mut buffer = [0u8; 1024];
    loop {
        match stdin.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let mut sent = 0;
                while sent < n {
                    match channel_in.write(&buffer[sent..n]) {
                        Ok(0) => break,
                        Ok(written) => {
                            sent += written;
                            let _ = channel_in.flush();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(_) => break,
        }
    }

    sess.set_blocking(true);
    let _ = channel.wait_close();
}

pub fn execute_collect(server: &Server, command: &str) -> String {
    let tcp = TcpStream::connect(format!("{}:{}", server.host, server.port))
        .expect("Failed to connect");

    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();

    if let Some(key) = &server.key_path {
        sess.userauth_pubkey_file(&server.user, None, Path::new(key), None).unwrap();
    } else if let Some(password) = &server.password {
        sess.userauth_password(&server.user, password).unwrap();
    } else {
        sess.userauth_agent(&server.user).unwrap();
    }

    let mut channel = sess.channel_session().unwrap();
    channel.exec(command).unwrap();

    let mut s = String::new();
    channel.read_to_string(&mut s).unwrap();

    s
}


pub fn interactive_multi_worker(
    server: Server,
    input_rx: std::sync::mpsc::Receiver<String>,
    output_tx: Sender<(String, String, String)>,
    color: String,
) {
    let tcp = TcpStream::connect(format!("{}:{}", server.host, server.port))
        .expect("Failed to connect");

    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();

    if let Some(key) = &server.key_path {
        sess.userauth_pubkey_file(&server.user, None, Path::new(key), None)
            .expect("Key auth failed");
    } else if let Some(password) = &server.password {
        sess.userauth_password(&server.user, password)
            .expect("Password auth failed");
    } else {
        sess.userauth_agent(&server.user)
            .expect("Agent auth failed");
    }

    if !sess.authenticated() {
        panic!("Auth failed");
    }

    let mut channel = sess.channel_session().unwrap();

    let (w, h) = terminal_size().unwrap_or((80, 24));
    channel.request_pty("xterm-256color", None, Some((w as u32, h as u32, 0, 0))).unwrap();
    channel.shell().unwrap();

    let mut buffer = [0u8; 1024];
    sess.set_blocking(false);

    loop {
        match channel.read(&mut buffer) {
            Ok(n) => {
                if n > 0 {
                    let text = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = output_tx.send((server.name.clone(), text, color.clone()));
                } else {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        while let Ok(mut cmd) = input_rx.try_recv() {
            if cmd == "\r" || cmd == "\n" {
                cmd = "\r".to_string();
            }
            let bytes = cmd.as_bytes();
            let mut sent = 0;
            while sent < bytes.len() {
                match channel.write(&bytes[sent..]) {
                    Ok(0) => break,
                    Ok(n) => {
                        sent += n;
                        let _ = channel.flush();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(_) => break,
                }
            }
        }

        if channel.eof() || !sess.authenticated() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let _ = output_tx.send((server.name.clone(), "DISCONNECTED_BY_JUNIE".to_string(), color));
    sess.set_blocking(true);

    channel.wait_close().unwrap();
}