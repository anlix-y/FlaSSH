use crate::models::{Group, Server};
use crate::storage;
use tokio::task;
use tokio::time::{timeout, Duration};
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use colored::*;
use rand::seq::SliceRandom;

const COLORS: &[&str] = &["red", "green", "yellow", "blue", "magenta", "cyan"];

pub fn add(name: String, servers: Vec<String>) -> Result<(), String> {
    if name == "all" {
        return Err("Name 'all' is reserved".to_string());
    }

    let mut groups = storage::group::load();
    if groups.iter().any(|g| g.name == name) {
        return Err(format!("Group '{}' already exists", name));
    }

    groups.push(Group { name, servers });
    storage::group::save(&groups);
    Ok(())
}

pub fn list() {
    let groups = storage::group::load();
    if groups.is_empty() {
        println!("No groups configured.");
        return;
    }

    for g in groups {
        println!("{:<15} {:?}", g.name, g.servers);
    }
}

pub fn remove(name: String) -> Result<(), String> {
    let mut groups = storage::group::load();
    let initial_len = groups.len();
    groups.retain(|g| g.name != name);

    if groups.len() == initial_len {
        return Err(format!("Group '{}' not found", name));
    }

    storage::group::save(&groups);
    Ok(())
}
pub async fn run_stream(group_name: String, command: String) {
    let groups = storage::group::load();
    let servers = storage::server::load();

    let group = match groups.iter().find(|g| g.name == group_name) {
        Some(g) => g,
        None => {
            println!("Group '{}' not found", group_name);
            return;
        }
    };

    let (tx, rx) = mpsc::channel();
    let mut rng = rand::thread_rng();

    for server_name in &group.servers {
        if let Some(server) = servers.iter().find(|s| &s.name == server_name).cloned() {
            let tx = tx.clone();
            let cmd = command.clone();
            let color = COLORS.choose(&mut rng).unwrap_or(&"cyan").to_string();

            tokio::spawn(async move {
                let (name, output) = run_with_retry(server, cmd).await;

                tx.send((name, output, color)).unwrap();
            });
        }
    }

    drop(tx);

    for (name, output, color) in rx {
        print!("[{}] {}", name.color(color), output);
    }
}

pub async fn run_with_retry(server: Server, cmd: String) -> (String, String) {
    let name = server.name.clone();

    for attempt in 1..=3 {
        let server_clone = server.clone();
        let cmd_clone = cmd.clone();

        let result = timeout(
            Duration::from_secs(5),
            task::spawn_blocking(move || {
                crate::ssh::execute_collect(&server_clone, &cmd_clone)
            })
        ).await;

        match result {
            Ok(join) => match join {
                Ok(output) => return (name, output),
                Err(_) => println!("[{}] thread error", name),
            },
            Err(_) => println!("[{}] timeout (try {})", name, attempt),
        }
    }

    (name, "Failed after retries\n".to_string())
}

pub fn interactive(group_name: String) {
    let groups = storage::group::load();
    let servers = storage::server::load();

    let group = match groups.iter().find(|g| g.name == group_name) {
        Some(g) => g,
        None => {
            println!("Group '{}' not found", group_name);
            return;
        }
    };

    let mut input_txs = Vec::new();
    let (output_tx, output_rx) = mpsc::channel();

    let mut rng = rand::thread_rng();

    use termion::raw::IntoRawMode;
    let _stdout = std::io::stdout().into_raw_mode().unwrap();

    for server_name in &group.servers {
        if let Some(server) = servers.iter().find(|s| &s.name == server_name).cloned() {
            let (tx, rx) = mpsc::channel();
            input_txs.push(tx);
            let output_tx = output_tx.clone();
            let color = COLORS.choose(&mut rng).unwrap_or(&"cyan").to_string();

            thread::spawn(move || {
                crate::ssh::interactive_multi_worker(server, rx, output_tx, color);
            });
        }
    }

    drop(output_tx);

    let (input_print_tx, input_print_rx) = mpsc::channel();
    let input_txs_clone = input_txs.clone();

    let focus_index_shared = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let focus_index_for_input = focus_index_shared.clone();
    
    let current_line_shared = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let current_line_for_input = current_line_shared.clone();
    let current_line_for_main = current_line_shared.clone();

    thread::spawn(move || {
        use std::io::{Read, Write};
        let mut buffer = [0u8; 1024];
        loop {
            match std::io::stdin().read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let input = String::from_utf8_lossy(&buffer[..n]).to_string();
                    for c in input.chars() {
                        let cur_focus = focus_index_for_input.load(std::sync::atomic::Ordering::SeqCst);
                        if c == '\r' || c == '\n' {
                            let mut line = current_line_for_input.lock().unwrap();
                            let cmd = format!("{}\n", *line);
                            if cur_focus == 0 {
                                for tx in &input_txs_clone {
                                    let _ = tx.send(cmd.clone());
                                }
                            } else if cur_focus <= input_txs_clone.len() {
                                let _ = input_txs_clone[cur_focus - 1].send(cmd.clone());
                            }
                            let _ = input_print_tx.send(("\n".to_string(), line.clone()));
                            line.clear();
                        } else if c == '¡' || c == '⁄' || c == '€' || c == '‹' || c == '›' || c == 'ﬁ' || c == 'ﬂ' || c == '‡' || c == '·' {
                            // MacOS Option + 1-9 characters
                            let digit = match c {
                                '¡' => 1, '⁄' => 2, '€' => 3, '‹' => 4, '›' => 5,
                                'ﬁ' => 6, 'ﬂ' => 7, '‡' => 8, '·' => 9,
                                _ => 0
                            };
                            if digit > 0 {
                                let _ = input_print_tx.send(("alt_digit".to_string(), digit.to_string()));
                            }
                            continue;
                        } else if c == 'ß' {
                            // MacOS Option + s
                            let _ = input_print_tx.send(("alt_s".to_string(), String::new()));
                            continue;
                        } else if c == '\x1b' {
                            // Small delay to allow subsequent bytes to arrive
                            std::thread::sleep(std::time::Duration::from_millis(20));

                            let mut next_byte = [0u8; 1];
                            if std::io::stdin().read(&mut next_byte).is_ok() {
                                let b = next_byte[0];
                                if b == b'\t' {
                                    let _ = input_print_tx.send(("alt_tab".to_string(), String::new()));
                                    continue;
                                } else if b >= b'1' && b <= b'9' {
                                    let digit = (b - b'0') as usize;
                                    let _ = input_print_tx.send(("alt_digit".to_string(), digit.to_string()));
                                    continue;
                                } else if b == b's' {
                                    let _ = input_print_tx.send(("alt_s".to_string(), String::new()));
                                    continue;
                                }
                            }
                        } else if c == '\x03' {
                            // Ctrl+C
                            let mut line = current_line_for_input.lock().unwrap();
                            for tx in &input_txs_clone {
                                let _ = tx.send("\x03".to_string());
                            }
                            let _ = input_print_tx.send(("\x03".to_string(), String::new()));
                            line.clear();
                        } else if c == '\x08' || c == '\x7f' {
                            let mut line = current_line_for_input.lock().unwrap();
                            if !line.is_empty() {
                                line.pop();
                                let _ = input_print_tx.send(("\x08".to_string(), String::new()));
                            }
                        } else {
                            let mut line = current_line_for_input.lock().unwrap();
                            line.push(c);
                            let _ = input_print_tx.send(("char".to_string(), c.to_string()));
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut buffers: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
    let mut last_printed_server: Option<String> = None;
    let mut at_line_start = true;

    let mut active_servers: std::collections::HashSet<String> = group.servers.iter().cloned().collect();

    let server_names = group.servers.clone();
    
    // Buffer for all output from each server to allow sorting
    let mut session_history: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for name in &group.servers {
        session_history.insert(name.clone(), Vec::new());
    }

    let mut prompt_printed = false;

    loop {
        let cur_focus = focus_index_shared.load(std::sync::atomic::Ordering::SeqCst);

        if !prompt_printed || at_line_start {
            let prompt = if cur_focus == 0 {
                "[all] ".to_string()
            } else {
                format!("[{}] ", server_names[cur_focus - 1])
            };
            print!("\r{}", termion::clear::CurrentLine);
            let current_line = current_line_for_main.lock().unwrap().clone();
            print!("{}{}", prompt, current_line);
            let _ = std::io::stdout().flush();
            at_line_start = false;
            prompt_printed = true;
        }

        if let Ok((input_type, line_content)) = input_print_rx.try_recv() {
            if input_type == "char" {
                print!("{}", line_content);
                let _ = std::io::stdout().flush();
            } else if input_type == "\x08" {
                print!("\x08 \x08");
                let _ = std::io::stdout().flush();
            } else if input_type == "\n" {
                print!("\r\n");
                let _ = std::io::stdout().flush();
                // Buffer to store what we printed in this command session for sorting
                let mut current_session_outputs: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
                for name in &group.servers {
                    current_session_outputs.insert(name.clone(), Vec::new());
                }

                // Command sent, now wait for outputs sequentially
                let cur_focus = focus_index_shared.load(std::sync::atomic::Ordering::SeqCst);
                let target_server_name = if cur_focus > 0 && cur_focus <= server_names.len() {
                    Some(&server_names[cur_focus - 1])
                } else {
                    None
                };

                let mut sorted_names: Vec<_> = if cur_focus == 0 {
                    group.servers.clone()
                } else {
                    vec![server_names[cur_focus - 1].clone()]
                };
                
                for s_name in sorted_names {
                    if !active_servers.contains(&s_name) {
                        continue;
                    }

                    let color = buffers.get(&s_name).map(|(_, c)| c.clone()).unwrap_or_else(|| "cyan".to_string());
                    
                    // Wait a bit for output to start arriving
                    let start_wait = std::time::Instant::now();
                    let mut received_any = false;
                    
                    loop {
                        // Check for new input while waiting for output (e.g. Ctrl+C)
                        if let Ok((itype, _)) = input_print_rx.try_recv() {
                            if itype == "\x03" {
                                // Ctrl+C handled
                            }
                        }

                        // Check for new output
                        while let Ok((name, mut output, color_new)) = output_rx.try_recv() {
                            if output == "DISCONNECTED_BY_JUNIE" {
                                active_servers.remove(&name);
                                continue;
                            }
                            output = output.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
                            
                            if target_server_name.is_some() && target_server_name != Some(&name) {
                                // Buffer output from other servers for later when switching focus
                                let entry = buffers.entry(name.clone()).or_insert((String::new(), color_new));
                                entry.0.push_str(&output);
                                continue;
                            }

                            let entry = buffers.entry(name.clone()).or_insert((String::new(), color_new));
                            entry.0.push_str(&output);
                        }

                        let has_output = buffers.get(&s_name).map(|(b, _)| !b.is_empty()).unwrap_or(false);
                        
                        if has_output {
                            if !received_any {
                                if last_printed_server.is_some() && last_printed_server.as_ref() != Some(&s_name) {
                                    print!("\r\n");
                                }
                                received_any = true;
                                last_printed_server = Some(s_name.clone());
                            }

                            if let Some((buffer, _)) = buffers.get_mut(&s_name) {
                                let server_output = buffer.drain(..).collect::<String>();
                                let lines: Vec<&str> = server_output.split("\r\n").collect();
                                for (i, line) in lines.iter().enumerate() {
                                    if i == lines.len() - 1 && line.is_empty() {
                                        continue;
                                    }
                                    let prefix = format!("[{}] ", s_name.color(color.clone()));
                                    print!("{}{}\r\n", prefix, line);
                                    
                                    if let Some(history) = current_session_outputs.get_mut(&s_name) {
                                        history.push(line.to_string());
                                    }
                                    if let Some(history) = session_history.get_mut(&s_name) {
                                        history.push(line.to_string());
                                    }
                                }
                                let _ = std::io::stdout().flush();
                            }
                        }

                        if received_any {
                            thread::sleep(Duration::from_millis(100));
                             while let Ok((name, mut output, color_new)) = output_rx.try_recv() {
                                if output == "DISCONNECTED_BY_JUNIE" {
                                    active_servers.remove(&name);
                                    continue;
                                }
                                output = output.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
                                let entry = buffers.entry(name.clone()).or_insert((String::new(), color_new));
                                entry.0.push_str(&output);
                            }
                            
                            if buffers.get(&s_name).map(|(b, _)| b.is_empty()).unwrap_or(true) {
                                break;
                            }
                        } else {
                            if start_wait.elapsed() > Duration::from_secs(2) {
                                break;
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
                at_line_start = true;
                last_printed_server = None;
            } else if input_type == "alt_tab" {
                let next_focus = (focus_index_shared.load(std::sync::atomic::Ordering::SeqCst) + 1) % (server_names.len() + 1);
                focus_index_shared.store(next_focus, std::sync::atomic::Ordering::SeqCst);
                at_line_start = true;
                while input_print_rx.try_recv().is_ok() {}
            } else if input_type == "alt_digit" {
                let digit = line_content.parse::<usize>().unwrap_or(0);
                if digit <= server_names.len() {
                    focus_index_shared.store(digit, std::sync::atomic::Ordering::SeqCst);
                }
                at_line_start = true;
                while input_print_rx.try_recv().is_ok() {}
            } else if input_type == "alt_s" {
                print!("\r\n--- Output Grouped by Server ---\r\n");
                for s_name in &server_names {
                    if let Some(lines) = session_history.get(s_name) {
                        if lines.is_empty() { continue; }
                        let color = buffers.get(s_name).map(|(_, c)| c.clone()).unwrap_or("cyan".to_string());
                        for line in lines {
                            print!("[{}] {}\r\n", s_name.color(color.clone()), line);
                        }
                    }
                }
                print!("--------------------------------\r\n");
                at_line_start = true;
            } else if input_type == "\x03" {
                print!("^C\r\n");
                at_line_start = true;
                last_printed_server = None;
            }
        }

                // Also check for unexpected output (not triggered by our command)
                while let Ok((name, mut output, color)) = output_rx.try_recv() {
                    if output == "DISCONNECTED_BY_JUNIE" {
                        active_servers.remove(&name);
                        continue;
                    }
                    output = output.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
                    
                    let cur_focus = focus_index_shared.load(std::sync::atomic::Ordering::SeqCst);
                    let target_name = if cur_focus > 0 && cur_focus <= server_names.len() {
                        Some(&server_names[cur_focus - 1])
                    } else {
                        None
                    };

                    if target_name.is_some() && target_name != Some(&name) {
                        // Skip output from other servers when focused on one
                        continue;
                    }

                    if last_printed_server.as_ref() != Some(&name) {
                        if last_printed_server.is_some() {
                            print!("\r\n");
                        }
                        last_printed_server = Some(name.clone());
                    }

                    let lines: Vec<&str> = output.split("\r\n").collect();
                    for (i, line) in lines.iter().enumerate() {
                        if i == lines.len() - 1 && line.is_empty() {
                            continue;
                        }
                        let prefix = format!("[{}] ", name.color(color.clone()));
                        print!("{}{}\r\n", prefix, line);
                        if let Some(history) = session_history.get_mut(&name) {
                            history.push(line.to_string());
                        }
                    }
                    let _ = std::io::stdout().flush();
                    at_line_start = true;
                }

        if active_servers.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}