use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

use termion::raw::IntoRawMode;

use crate::models::Server;

pub fn execute(server: &Server, command: &str) {
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

    println!("{}@{}:\n{}", server.user, server.host, s);

    channel.wait_close().unwrap();
}

pub fn interactive(server: &Server) {
    let tcp = TcpStream::connect(format!("{}:{}", server.host, server.port))
        .expect("Failed to connect");

    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();

    if let Some(key) = &server.key_path {
        sess.userauth_pubkey_file(
            &server.user,
            None,
            Path::new(key),
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

    channel.request_pty("xterm", None, None).unwrap();
    channel.shell().unwrap();

    let mut stdout = std::io::stdout().into_raw_mode().unwrap();
    let mut stdin = std::io::stdin();

    let mut channel_stream = channel.stream(0);

    loop {
        let mut buffer = [0u8; 1024];

        match channel_stream.read(&mut buffer) {
            Ok(n) if n > 0 => {
                stdout.write_all(&buffer[..n]).unwrap();
                stdout.flush().unwrap();
            }
            _ => {}
        }

        match stdin.read(&mut buffer) {
            Ok(n) if n > 0 => {
                channel.write_all(&buffer[..n]).unwrap();
                channel.flush().unwrap();
            }
            _ => {}
        }

        if channel.eof() {
            break;
        }
    }

    channel.wait_close().unwrap();
}