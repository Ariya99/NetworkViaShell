use network_via_shell_ssh::{SshProfile, SshTunnelManager};
use std::{env, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};

fn integration_profile(port: u16, identity_file: String) -> SshProfile {
    SshProfile {
        host: "127.0.0.1".into(),
        port: env::var("NETWORK_VIA_SHELL_SSH_PORT")
            .expect("integration SSH port")
            .parse()
            .expect("valid integration SSH port"),
        username: env::var("NETWORK_VIA_SHELL_SSH_USER").expect("integration SSH username"),
        local_socks_port: port,
        identity_file: Some(identity_file.into()),
    }
}

#[tokio::test]
async fn disposable_openssh_server_proves_socks5_and_shutdown_lifecycle() {
    // The Linux CI job provisions an isolated sshd and sets these variables.
    // Local and Windows test runs remain self-contained when it is unavailable.
    let Ok(identity_file) = env::var("NETWORK_VIA_SHELL_SSH_KEY") else {
        return;
    };
    if env::var("NETWORK_VIA_SHELL_SSH_INTEGRATION").as_deref() != Ok("1") {
        return;
    }

    let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_address = echo_listener.local_addr().unwrap();
    let echo_task = tokio::spawn(async move {
        let (mut stream, _) = echo_listener.accept().await.unwrap();
        let mut buffer = [0_u8; 32];
        let count = stream.read(&mut buffer).await.unwrap();
        stream.write_all(&buffer[..count]).await.unwrap();
    });

    let socks_port = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let profile = integration_profile(socks_port, identity_file);
    let manager = SshTunnelManager::default();

    let endpoint = manager.start(profile.clone()).await.unwrap();
    assert_eq!(endpoint, format!("127.0.0.1:{socks_port}"));

    // A manager cannot accidentally replace its active child with another tunnel.
    let second_start = manager.start(profile).await;
    assert!(second_start.is_err());

    let mut socks = TcpStream::connect(endpoint.parse::<SocketAddr>().unwrap())
        .await
        .unwrap();

    // SOCKS5 no-authentication negotiation.
    socks.write_all(&[5, 1, 0]).await.unwrap();
    let mut greeting = [0_u8; 2];
    socks.read_exact(&mut greeting).await.unwrap();
    assert_eq!(greeting, [5, 0]);

    // CONNECT to the loopback echo service on the SSH server side.
    let target_port = echo_address.port().to_be_bytes();
    socks
        .write_all(&[
            5,
            1,
            0,
            1,
            127,
            0,
            0,
            1,
            target_port[0],
            target_port[1],
        ])
        .await
        .unwrap();
    let mut connect_reply = [0_u8; 10];
    socks.read_exact(&mut connect_reply).await.unwrap();
    assert_eq!(&connect_reply[..4], &[5, 0, 0, 1]);

    let payload = b"network-via-shell";
    socks.write_all(payload).await.unwrap();
    let mut echoed = vec![0_u8; payload.len()];
    socks.read_exact(&mut echoed).await.unwrap();
    assert_eq!(echoed, payload);
    drop(socks);
    echo_task.await.unwrap();

    manager.stop().await.unwrap();
    manager.stop().await.unwrap();

    // Give the OS a short interval to release the listener after child exit.
    let mut unavailable = false;
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", socks_port)).await.is_err() {
            unavailable = true;
            break;
        }
        sleep(Duration::from_millis(25)).await;
    }
    assert!(unavailable, "SOCKS listener remained open after stop");
    assert!(timeout(
        Duration::from_secs(1),
        TcpStream::connect(("127.0.0.1", socks_port))
    )
    .await
    .is_ok());
}
