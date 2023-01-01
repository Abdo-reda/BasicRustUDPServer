use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::{thread, vec};
use std::time::Duration;
use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr};
use serde::{Deserialize, Serialize};
use local_ip_address::local_ip;
use std::mem::transmute;
use std::time::Instant;
use flume;
use std::env;
use std::io::{self, Seek, SeekFrom};

const BUFFER_SIZE: usize = 256;

const MACHINE_ONE:&'static str = "10.7.57.90:8080";
const MACHINE_TWO:&'static str = "10.7.57.90:8081";
const MACHINE_THREE:&'static str = "10.7.57.90:8082";

const SERVER_LIST: &'static [&'static str] = &[MACHINE_ONE, MACHINE_TWO, MACHINE_THREE];

const NUMBER_SERVERS: usize = 3;

//const PORT : u16 = 7676;

const CLIENT_UPDATE_SECONDS : u64 = 20;
const MIDDLEWARE_TIMEOUT:u64 = 3;
const SERVER_SLEEP_TIME:u64 = 10;

#[derive(Serialize, Deserialize)]
struct Message{
    m_Type: u8,
    m_ID: String,
    remote_Ref: String, 
    local_Ref: String,
    op_Id: u32,
    op_Arg: Vec<u8>,
}


struct Client_Message{
    threadId: i32,
    messageId: i64,
    duration: Duration,
}
#[derive(Clone)]
struct Client_Log{
    threadId: i32,
    sentRequest: i64,
    fullfilledRequest: i64,
    totalLatency: Duration,
    totalDuration: Duration,
}

type TempT = Client_Message;
type LogT = Client_Log;


fn logging_thread(loggingRecv:Receiver<LogT>, fileName:String){

    let mut file = File::create(fileName).expect("couldn't create file!!!");
    let n:usize = 500;
    let mut infoVec:Vec<Client_Log> = vec![ Client_Log { threadId: (0), sentRequest: (0), fullfilledRequest: (0), totalLatency: (Duration::new(0,0)), totalDuration: (Duration::new(0,0)) };n];

    loop{

        //----Recieve Log
        let (value) = match (loggingRecv.try_recv()) {
            Ok( TempT ) => (TempT), 
            Err(error) => ( Client_Log{
                threadId: -1,
                sentRequest: -1,
                fullfilledRequest: -1,
                totalLatency: Duration::new(0,0),
                totalDuration: Duration::new(0,0),
            }),
        };

        //----Print/Write the log to the file
        if (value.threadId != -1){
            //---add thread info and truncate.
            infoVec[value.threadId as usize] = value.clone();
            file.set_len(0);
            file.seek(SeekFrom::Start(0));

            let mut TOTAL_SENT:u128 = 0;
            let mut TOTAL_FULLFILLED = 0;
            let mut TOTAL_FAILED = 0;
            let mut AVG_LATENCY = 0;
            let mut AVG_THROUGHPUT:f64 = 0.0;

            for  x in infoVec.to_owned()  {
                
 

                let tempFullfilled:u128 = x.fullfilledRequest.try_into().unwrap();
                let tempSent:u128 = (x.sentRequest+1).try_into().unwrap();
                let mut tempAvgLatency: u128 = (x.totalLatency.as_millis().checked_div(tempFullfilled)).unwrap_or(0) ;
                let mut tempAvgThroughput: f64 =   (tempFullfilled as f64) * 1000.0 / (x.totalDuration.as_millis() as f64) ;
                let mut failedRequests: i64 = x.sentRequest - x.fullfilledRequest + 1;
                let mut tempString:String = "Thread: ".to_owned();
                tempString.push_str(&x.threadId.to_string());
                tempString.push_str(", Sent: ");
                tempString.push_str(&(x.sentRequest+1).to_string());
                tempString.push_str(", Fullfilled: ");
                tempString.push_str(&x.fullfilledRequest.to_string());
                tempString.push_str(", failedRequests: ");
                tempString.push_str(&failedRequests.to_string());
                tempString.push_str(", totalLatency: ");
                tempString.push_str(&x.totalLatency.as_millis().to_string());
                tempString.push_str(", totalDuration: ");
                tempString.push_str(&x.totalDuration.as_millis().to_string());
                tempString.push_str(", avgLatency: ");
                tempString.push_str(& (tempAvgLatency).to_string());
                tempString.push_str(", avgThroughput: ");
                tempString.push_str(&tempAvgThroughput.to_string());
                tempString.push_str("\n");

                file.write(tempString.as_bytes());

                TOTAL_SENT += (tempSent);
                TOTAL_FULLFILLED += tempFullfilled;
                TOTAL_FAILED += (tempSent-tempFullfilled);
                AVG_LATENCY += tempAvgLatency;
                AVG_THROUGHPUT += tempAvgThroughput;

            }
            

    
            let mut tempString:String = "TOTAL SENT: ".to_owned();
            tempString.push_str(&(TOTAL_SENT).to_string());
            tempString.push_str(", TOTAL FULLFILLED: ");
            tempString.push_str(&TOTAL_FULLFILLED.to_string());
            tempString.push_str(", TOTAL FAILED: ");
            tempString.push_str(&TOTAL_FAILED.to_string());
            tempString.push_str(", AVG LATENCY: ");
            tempString.push_str(& (AVG_LATENCY/500).to_string());
            tempString.push_str(", TOTAL THROUGHPUT: ");
            tempString.push_str(&AVG_THROUGHPUT.to_string());

            file.write(tempString.as_bytes());
        
        }

           
           //file.write_all(  ("Thread: ".to_string() + &value.threadId.to_string() + &", Sent: ".to_string() + &value.sentRequest.to_string() + &", Fullfilled: ".to_string() + &value.fullfilledRequest.to_string() + &", avgDuration: ".to_string() + &value.avgDuration.as_micros().to_string() + &", tempDuration".to_string() + &value.tempDuration.as_micros().to_string()).as_bytes() + &"\n".to_string() ).expect("couldn't write to the file !!!!");
      
    }

}


fn send_random_requests(threadSendMiddle: Sender<TempT>, threadSendLog: Sender<LogT>, threadRecvMiddle:Receiver<TempT> ,i: i32){

    //println!("Client App {} is running ... ", i);
    let mut tempCount:i64 = 0;
    let mut CLIENT_UPDATE_DURATION : Duration = Duration::new(CLIENT_UPDATE_SECONDS, 0);

    let mut tempFullfilledCount:i64 = 0;
    let mut avgDuration:Duration = Duration::new(0,0);
    //let mut messageTimers: Vec<Duration> = Vec::new(); //or make it a part of the struct ....

    let now = Instant::now();

    loop{
     
        //overflow ?! u64::MAX
        /*if(tempCount >= i64::MAX){
            tempCount = 0;
        }*/

     
        //---Construct message
        let mut threadMessage = Client_Message{
            threadId: i,
            messageId: tempCount,
            duration: now.elapsed().clone(),
        };

        //thread::sleep(Duration::from_millis(1000));

        //---Send message
        threadSendMiddle.send(threadMessage).expect("couldn't send message to middleware");
       // println!(" Start {:.2?}", now.elapsed());
        //---Try to receive
        let (mut value) = match (threadRecvMiddle.recv()) {
            Ok( TempT ) => (TempT), 
            Err(error) => ( Client_Message{
                threadId: -1,
                messageId: -1,
                duration: Duration::new(0, 0)
                }),
        };

        if(value.threadId != -1 && value.messageId != -1){
            //---------------------------------------AN OVERFLOW HAPPENS WHEN SUBSTRACTING DUATIONS. SOMETIMES??????
                //checked_sub !!! none is returned if overflow happens.
            //println!("thread{}, id{}, {:.2?}, {:.2?}", value.threadId, value.messageId, now.elapsed(), value.duration);
            value.duration = (now.elapsed().checked_sub(value.duration)).unwrap_or(Duration::new(0, 0)); 
           
            if(value.duration.as_nanos() != 0){
                avgDuration += value.duration;
                tempFullfilledCount += 1;
            }else{
                println!("i{}, thread{}, id{} error?!", i, value.threadId, value.messageId);
               
            }
         
            //messageTimers[value.threadId as usize] = now.elapsed() - messageTimers[value.threadId as usize];
            //println!("Thread {}, Recieved Message {}, Elapsed: {:.2?}", value.threadId, value.messageId, value.duration);
        }

            
        //-----Update the Metrics
        let tempNow = now.elapsed();
        if(tempNow >= CLIENT_UPDATE_DURATION){
            println!("thread {} updating logs ...", i);
            let mut threadLog = Client_Log{
                threadId: i,
                sentRequest: tempCount,
                fullfilledRequest: tempFullfilledCount,
                totalLatency: avgDuration,
                totalDuration: tempNow,
            };
            CLIENT_UPDATE_DURATION =  CLIENT_UPDATE_DURATION + Duration::new(CLIENT_UPDATE_SECONDS, 0);
            threadSendLog.send(threadLog).expect("couldn't send to logging thread");
        }

        tempCount = tempCount + 1;
       
    }

}



fn do_operation(send_message:&Message, socket:&UdpSocket) -> (u8,Vec<u8>){

    //---Send Messege + Marshalling
    let send_bytes: Vec<u8> = bincode::serialize(&send_message).unwrap(); 
    socket.set_read_timeout(Some(Duration::new(MIDDLEWARE_TIMEOUT, 0)));
    socket.send_to(&send_bytes, send_message.remote_Ref.to_string()).expect("couldn't send message");

    //---Get Reply + UnMarshalling
    let mut reply_buf = [0; BUFFER_SIZE];
    let temp_reply = socket.recv_from(&mut reply_buf);
    let mut reply_mes: Message;

    //---Simple Error Handling,
    let (num_bytes, src) = match temp_reply {
        Ok((usize, SocketAddr)) => (usize, SocketAddr), 
        Err(error) => (0, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9999)),
    };

    if num_bytes != 0 {
        reply_mes = bincode::deserialize(&reply_buf).expect("couldn't deserilize reply");
        println!("{}", reply_mes.m_ID);
        (1,reply_mes.op_Arg)
    }else{
        println!("Error, Problem recieving server reply!!");
        (0,vec![])
    }

}


fn middlware_thread(middlewareSenders:Vec<Sender<TempT>>, middlewareRecv: Receiver<TempT>, PORT_SENDER:u16, Servers_State: Arc<Mutex<Vec<bool>>>){
    println!("Middleware is running ... ");

    //--------------Client Middleware    

    //---------Keep track
    let mut Server_Status: Vec<bool> = vec![true; NUMBER_SERVERS];

    //----------Bind to a socket to send from and a socket to com
    let socket = UdpSocket::bind( ("0.0.0.0:").to_string() + &PORT_SENDER.to_string()).expect("couldn't bind to address");
    //socket.set_read_timeout(Duration::from_millis(3000)).expect("set_read_timeout call failed");

    let my_local_ip = local_ip().unwrap();


    //---------Create a random messege
    let mut message = Message{
        m_Type: 0,
        m_ID: ("").to_string(),
        remote_Ref: ("").to_string(),
        local_Ref: ("").to_string(),
        op_Id: 0,
        op_Arg: vec![],
    };


    //-----------Loop and keep sending messeges using do_operation.
    let mut cur_server = 0; //---This keeps track of which server to send to used in a very simple load balancing robin algorithm
    let mut request_id = 0; //---Unique id per mesage sent.
    loop{


        let (mut value) = match (middlewareRecv.try_recv()) {
            Ok( TempT ) => (TempT), 
            Err(error) => ( Client_Message{
                threadId: -1,
                messageId: -1,
                duration: Duration::new(0, 0),
                }),
        };

   
        //----------Handle Requestremote_Ref
        if(value.threadId != -1){
            println!("Sending Messege from Client {}", value.threadId);

            //-----Editing the message to be sent.
            //Preform check here!
            if Servers_State.lock().unwrap()[cur_server]  == false {
                cur_server = (cur_server + 1)%NUMBER_SERVERS;
            }
            message.remote_Ref = SERVER_LIST[cur_server].to_string();  //pick this server.
            message.local_Ref = my_local_ip.to_string() + ":" + &PORT_SENDER.to_string();
            message.m_ID = my_local_ip.to_string() + ":" + &PORT_SENDER.to_string() + ":" + &request_id.to_string(); //messege id.
            //let bytes: [u8; 4] = unsafe { transmute(value.threadId.to_be()) };
           // message.op_Arg.extend_from_slice(&bytes);
            
            
            //-----Sending the message to server and getting the response.
            let  (mut successFlag, mut temp_returned_arg) = do_operation(&message, &socket);
            let str = String::from_utf8(temp_returned_arg).expect("Found invalid UTF-8");
            println!("Relpy: {}", str);
            
            //-----Incrementing stuff
            cur_server = (cur_server + 1)%NUMBER_SERVERS;
            request_id = request_id + 1;

            //-----Replying to client_threads
            if(successFlag == 0){
                value.messageId = -1;
            }
            middlewareSenders[ (value.threadId as usize)].send(value).expect("couldn't send message to thread");
            
        }

        
    }
  
 
}


fn middleware_communicate(PORT_MIDDLEWARE:u16, Servers_State: Arc<Mutex<Vec<bool>>>){

    let socket = UdpSocket::bind( ("0.0.0.0:").to_string() + &PORT_MIDDLEWARE.to_string()).expect("couldn't bind to address");

    let now = Instant::now();
    let mut start:Duration = now.elapsed();
    let mut flagDown = false;
    loop{
        let mut reply_buf = [0; BUFFER_SIZE];
        let temp_reply = socket.recv_from(&mut reply_buf);
        let mut reply_mes: Message;
    
        //---Simple Error Handling,
        let (num_bytes, src) = match temp_reply {
            Ok((usize, SocketAddr)) => (usize, SocketAddr), 
            Err(error) => (0, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9999)),
        };
    
        if num_bytes != 0 {
            reply_mes = bincode::deserialize(&reply_buf).expect("couldn't deserilize reply");
            if (reply_mes.op_Id == 2) {      //handling announcement
                println!("Recieved Announcment!");
                Servers_State.lock().unwrap()[reply_mes.op_Arg[0] as usize] = false;
               // flagDown=true;
                start=now.elapsed();
            }
        }

        thread::sleep(Duration::from_millis(SERVER_SLEEP_TIME*1000));
        *Servers_State.lock().unwrap() = vec![true; NUMBER_SERVERS];
        /* 
        if( (now.elapsed() - start > Duration::new(SERVER_SLEEP_TIME,0)) && (flagDown) ){
            //all servers are allowed
            flagDown=false;
            *Servers_State.lock().unwrap() = vec![true; NUMBER_SERVERS];
            println!("Update Server States ...")
        }*/
        

    }
  

}

fn main() {

    //-----Variables and stuff
    let args: Vec<String> = env::args().collect();
    let PORT_SENDER = (&args[1]).parse::<u16>().unwrap();
    let PORT_MIDDLEWARE = (&args[2]).parse::<u16>().unwrap();
    let fileName = (&args[3]).parse::<String>().unwrap();
    let Servers_State = Arc::new(Mutex::new(vec![true; NUMBER_SERVERS]));
    let Servers_State_Clone0 = Servers_State.clone();
    let Servers_State_Clone1 = Servers_State.clone();

    //Middleware channels, This will be mutliple producers (threads) and 1 consumer (middleware)
    let (threadSend, middlewareRecv) = mpsc::channel();

    //Logs channels, 
    let (threadSendLog, LogRecv):(Sender<LogT>, Receiver<LogT>) = mpsc::channel();

    //THis will be senders and recievesr from middleware to threads.
    const n:usize = 500;
    let mut middlewareSenders: Vec<Sender<TempT>> = Vec::new();

    //Creating the logging thread
    thread::spawn(move || logging_thread(LogRecv, fileName));

    //Creating the middleware communication thread
    thread::spawn(move || middleware_communicate(PORT_MIDDLEWARE, Servers_State_Clone0));

    //Creating the client applications ...
    for k in 0..n{
      
        let ( tempSender, tempReciever )  = mpsc::channel::<TempT>();
        middlewareSenders.push(tempSender); 

        let tempSenderClone = threadSend.clone();
        let tempSenderLogClone = threadSendLog.clone();
        //let tempRecvClone = threadRecv.clone();
        //let tempRecv = threadRecievers.pop().unwrap();

        thread::spawn(move || {
            send_random_requests(tempSenderClone, tempSenderLogClone, tempReciever, k.try_into().unwrap());
        });
    }

    //Creating the middleware .... 
    thread::spawn(move || middlware_thread(middlewareSenders,middlewareRecv, PORT_SENDER, Servers_State_Clone1));

    //Making sure that the parent processes doesn't terimante.
    loop{

    }

}



