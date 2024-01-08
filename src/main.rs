use std::collections::HashMap;
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};

#[derive(Default)]
struct ChatState {
    users: HashMap<u32, String>,
}

fn give_me_default<T>() -> T
where
    T: Default,
{
    Default::default()
}

#[tokio::main]
async fn main() {
    let value = give_me_default::<i32>();
    let listener = TcpListener::bind("localhost:8000").await.unwrap();
    let (tx, _) = broadcast::channel(10);
    let chat_state = Mutex::new(ChatState::default());

    let mut nth_user = 0;

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let chat_state = Mutex::new(ChatState::default());
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // Ask for username
            nth_user += 1;
            println!("Enter your username?");
            let mut user = String::new();
            io::stdin()
                .read_line(&mut user)
                .expect("Failed to read username");

            {
                let mut state = chat_state.lock().await;
                state.users.insert(nth_user, user.trim().to_string());
            }

            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0 {
                            // User disconnected
                            let mut state = chat_state.lock().await;
                            state.users.remove(&nth_user);
                            break;
                        }

                        let message = format!("{}: {}", user.trim(), line);
                        tx.send((message.clone(), addr)).unwrap();
                        line.clear();
                    }
                    result = rx.recv() => {
                        if let Ok((msg, other_addr)) = result {
                            if addr != other_addr {
                                writer.write_all(msg.as_bytes()).await.unwrap();
                            }
                        }
                    }
                }
            }
        });
    }
}
