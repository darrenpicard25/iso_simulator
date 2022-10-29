use std::time::{Duration, Instant};

use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    runtime::Builder,
    time::{interval, sleep},
};

use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of clients wanted for connection. Cannot exceed available cores
    #[arg(short = 'c', long, default_value_t = 1)]
    client_count: u8,
    /// Number of requests to send per client
    #[arg(long)]
    request_count: usize,

    /// Number of Requests to send per second
    #[arg(short = 'r', long)]
    rate: usize,
}

struct Client {
    client_id: usize,
    connection: TcpStream,
    request_count: usize,
    rate: usize,
}

impl Client {
    async fn connect(client_id: usize, request_count: usize, rate: usize) -> Self {
        let connection = tokio::net::TcpStream::connect("localhost:8006")
            .await
            .unwrap();

        println!("Client {client_id} connected to localhost:8006");

        Self {
            client_id,
            connection,
            request_count,
            rate,
        }
    }

    async fn run_load(mut self) {
        let mut current_count = 0;
        let (_reader, mut writer) = self.connection.split();

        let mut interval = interval(Duration::from_nanos(1_000_000_000 / self.rate as u64));
        let hex_string = "011630313030767b44012861b00a3136353339353633303039393930303030393030303030303030303030303030313530303030303030303030313530303039333031333339303236313030303030303031323636393038333930323039333030393330303932393538313239303030363939393638313332353339353633303039393930303030393d323531303230313030303030363934303830303030313030303135462d353831322d43414e2020202020454154494e4720504c414345532c205245535441555220544f524f4e544f2020202020202043414e303037523830303254563132343132344069bef5f11420613032313030303030303030303038303031323439303231303030394d43534356304a5246";
        let buffer = hex::decode(hex_string).unwrap();
        let start_time = Instant::now();

        while self.request_count > current_count {
            interval.tick().await;
            writer.write(&buffer).await.unwrap();

            writer.flush().await.unwrap();

            current_count += 1;
        }

        println!(
            "Client {} successfully wrote {} messages to connection in {:?}",
            self.client_id,
            current_count,
            start_time.elapsed()
        );

        sleep(Duration::from_secs(10)).await;
    }
}

fn main() {
    let args = Args::parse();

    let available_clients = std::thread::available_parallelism().unwrap().get();

    if args.client_count >= available_clients as u8 {
        eprintln!(
            "Client count {} exceeds number of available cores: {}",
            args.client_count, available_clients
        );
        return;
    }

    prep_clients(args);
}

fn prep_clients(args: Args) {
    let mut threads = Vec::new();
    for client_id in 0..args.client_count {
        let thread = std::thread::spawn(move || {
            let runtime = Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .unwrap();

            runtime.block_on(async {
                let client =
                    Client::connect(client_id as usize, args.request_count, args.rate).await;

                client.run_load().await;
            })
        });

        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }
}
