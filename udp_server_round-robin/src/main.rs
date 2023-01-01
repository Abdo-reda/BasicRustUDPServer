use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::{thread, env};
use std::sync::{mpsc, Arc, Mutex};
use String;
use rand::Rng;
use std::time::{Duration, Instant};

mod func;
mod lib;

const MACHINE_ONE:&'static str = "10.7.57.90:8080";
const MACHINE_TWO:&'static str = "10.7.57.90:8081";
const MACHINE_THREE:&'static str = "10.7.57.90:8082";
const SERVER_LIST: &'static [&'static str] = &[MACHINE_ONE, MACHINE_TWO, MACHINE_THREE];
const NUMBER_SERVERS: usize = 3;


const CLIENT_ONE:&'static str = "10.7.57.41:8080";
const CLIENT_TWO:&'static str = "10.7.57.41:8081";
const CLIENT_LIST: &'static [&'static str] = &[CLIENT_ONE, CLIENT_TWO];
const NUMBER_CLIENTS: usize = 2;

const SLEEP_TIME: u64 = 10000;
const START_ELECTION_PROBABILITY: u32 = 20;
const TRY_START_ELECTION_FREQ: u64 = 2000;
const WORKER_COUNT: usize = 5;
const BUFFER_SIZE: usize = 256;


fn handel_request(mut mes: func::Message, src: SocketAddr, ID: Arc<u8>) // despatcher to handle functions 
{
    match mes.op_Id{
        0=>func::handel_normal(mes, src),
        1=>func::handle_elect(mes, src, ID),
        _=>func::handel_normal(mes, src),
      }
}

fn election_trigger(ID: Arc<u8>, rx: mpsc::Receiver<usize>, servers_state: Arc<Mutex<Vec<bool>>>)
{
    let socket = UdpSocket::bind("0.0.0.0:0").expect("couldn't bind to address");
    let mut rng = rand::thread_rng();

    loop 
    {
        //stop trying to start election if a server is already down
        let value = match rx.try_recv() 
        {
            Ok(usize) => usize, 
            Err(error) => NUMBER_SERVERS
        };
        if value < NUMBER_SERVERS
        {
            servers_state.lock().unwrap()[value] = false;
            thread::sleep(Duration::from_millis(SLEEP_TIME));
            servers_state.lock().unwrap()[value] = true;
        }

        //attempt to start election every TRY_START_ELECTION_FREQ msec
        thread::sleep(Duration::from_millis(TRY_START_ELECTION_FREQ));

        //election
        let rnd: u32 = rng.gen();
        let elect: bool = (rnd % START_ELECTION_PROBABILITY) == 0;

        if elect
        {
            let elect_msg = func::Message{
                m_Type: 0,
                m_ID: ("").to_string(),
                remote_Ref: ("").to_string(),
                local_Ref: SERVER_LIST[*ID as usize].to_string(),
                op_Id: 1,
                op_Arg: vec![*ID, rng.gen()],
            };

            let serial: Vec<u8> = bincode::serialize(&elect_msg).unwrap();

            socket.send_to(&serial, SERVER_LIST[((*ID as usize)+1)%NUMBER_SERVERS].to_string()).expect("couldn't send message");
        }
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let ID_ref = Arc::new(args[1].parse::<usize>().unwrap() as u8);
    println!("Server ID = {}", *ID_ref);

    //start logging
    let file_name = args[2].parse::<String>().unwrap();
    println!("file name: {}", file_name);
    let (tx_log, rx_log) = mpsc::channel();
    thread::spawn(||{
        func::server_log(rx_log, file_name);
    });

    let servers_state = Arc::new(Mutex::new(vec![true, true, true]));

    //start house keeping on a seperate thread to handle load balancing and election
    let ID_ref_clone1 = ID_ref.clone();
    let servers_state_clone = servers_state.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(||{
        election_trigger(ID_ref_clone1, rx, servers_state_clone);
    });

    //creates a udpsocket at this ip and port
    let socket = UdpSocket::bind(("0.0.0.0:808").to_string() + &ID_ref.to_string()).expect("couldn't bind to address");
    //create threadpool to handel requests
    let pool = lib::ThreadPool::new(WORKER_COUNT);

    let mut request_count: i64 = 0; //counter to get request/sec (load)
    let one = Duration::new(1, 0);
    loop
    {
        let now = Instant::now();
        while now.elapsed() < one
        {
            //recieve message, this is a blocking function.
            let (mes, src) = func::get_request(&socket);

            //handel announcement message (special case)
            if mes.op_Id == 2 
            {
                //stop election trigger
                let _ = tx.send(mes.op_Arg[0] as usize);
                if mes.op_Arg[0] == *ID_ref
                {
                    println!("I am down");
                    //send special value (-1) to logging
                    let _ =tx_log.send(-1);
                    thread::sleep(Duration::from_millis(SLEEP_TIME));
                    println!("I am back up");
                }
                else 
                {    
                    println!("A server is down, stoping election");
                }
            }
            else
            {
                //count requests every received in 1 sec
                request_count += 1;

                //send job to pool to be handeled 
                let src_clone = src.clone();
                let ID_ref_clone2 = ID_ref.clone();
                pool.execute(move || // excute the function in multi-threading 
                    {
                        handel_request(mes, src_clone, ID_ref_clone2);
                    });

                //check load for balancing
            }
        }
        //send requests/sec to loging
        let _ =tx_log.send(request_count);
        request_count = 0;
    }
}