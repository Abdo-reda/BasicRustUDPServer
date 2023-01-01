use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::{thread, convert::TryInto};
use serde::{Deserialize, Serialize};
use String;
use rand::Rng;
use std::sync::{mpsc, Arc, Mutex};
use std::fs::File;
use std::io::{Write, Seek, SeekFrom};
use chrono::Utc;

use crate::SERVER_LIST;
use crate::NUMBER_SERVERS;
use crate::CLIENT_LIST;
use crate::NUMBER_CLIENTS;
use crate::BUFFER_SIZE;

// 1 struct of type message and the message has 
//type of message / message ID (unique id to test sending and receving of a message/ remote reference(server)/ operation ID/ arguments ( array of bytes))

#[derive(Serialize, Deserialize)]
pub struct Message{
    pub m_Type: u8, // request: 0, reply: 1
    pub m_ID: String,
    pub remote_Ref: String,
    pub local_Ref: String,
    pub op_Id: u32, // normal: 0, elect: 1, announce: 2, balance load: 3 // different messages to the server 
    pub op_Arg: Vec<u8>,
}

pub fn get_request(socket: &UdpSocket) -> (Message, SocketAddr)
{
    let mut buf = [0; BUFFER_SIZE];
    let (num_bytes, src) = socket.recv_from(&mut buf).expect("couldn't receive data");
    let mes: Message = bincode::deserialize(&buf).unwrap();
    (mes, src)
}

fn send_reply(socket: &UdpSocket, message: Message, src: SocketAddr)
{
    let bytes = bincode::serialize(&message).unwrap(); 
    socket.send_to(&bytes, src).expect("couldn't send message");
}

pub fn handel_normal(mut mes: Message, src: SocketAddr)
{
    //.... Process request .....
    //thread::sleep(Duration::from_millis(1000));
    let socket = UdpSocket::bind("0.0.0.0:0").expect("couldn't bind to address");
    
    println!("m_ID =  {}", mes.m_ID);

    let c = "hello from server";
    
    mes.op_Arg = c.as_bytes().to_vec();

    //.... End Process request .....
    send_reply(&socket, mes, src);
    println!("execution done\n")
}

pub fn handle_elect(mut mes: Message, src: SocketAddr, my_ID: Arc<u8>)
{
    //---- Processing Elect
    println!("Handling election...");
    let socket = UdpSocket::bind("0.0.0.0:0").expect("couldn't bind to address");
    let mut rng = rand::thread_rng();
    
    if mes.op_Arg[0] == *my_ID
    {
        //determine who will fail based on node.priority
        let mut max = 0;
        let mut argmax: u8 = 0;
        let mut id;
        let mut value;
        for i in 0..(mes.op_Arg.len()/2)
        {
            id = mes.op_Arg[i*2];
            value = mes.op_Arg[i*2 + 1];
            if value > max
            {
                max = value;
                argmax = id;
            }
        }
        println!("Server chosen: {}", argmax);
        println!("Chosen value: {}", max);
        //broadcast announcment msg
        let announc_msg = Message{
            m_Type: 0,
            m_ID: ("").to_string(),
            remote_Ref: ("").to_string(),
            local_Ref: SERVER_LIST[*my_ID as usize].to_string(),
            op_Id: 2,
            op_Arg: vec![argmax],
        };
        let serial = bincode::serialize(&announc_msg).unwrap();
        
        for i in 0..NUMBER_SERVERS
        {
            socket.send_to(&serial, SERVER_LIST[i].to_string()).expect("couldn't send announcement message");
        }
        for i in 0..NUMBER_CLIENTS
        {
            socket.send_to(&serial, CLIENT_LIST[i].to_string()).expect("couldn't send announcement message");
        }
    }
    else
    {
        mes.op_Arg.append(&mut vec![*my_ID, rng.gen()]);
        let serial = bincode::serialize(&mes).unwrap();

        socket.send_to(&serial, SERVER_LIST[((*my_ID as usize)+1)%NUMBER_SERVERS].to_string()).expect("couldn't send election message");
    }
}

pub fn server_log(rx: mpsc::Receiver<i64>, file_name: String)
{
    let mut file = File::create(file_name).expect("couldn't create file!!!");

    let mut str: String;
    let mut load: i64;
    let mut avg_load: i64 = 0;
    let mut i = 0;
    loop 
    {
        if i >= 100
        {
            let _ = file.seek(SeekFrom::Start(0));
            i = 0;
        }
        load = rx.recv().unwrap();
        if load == -1
        {
            str = ("Server down at time: ").to_owned() + &Utc::now().to_string() + &("\n").to_owned();
        }
        else
        {
            avg_load = (avg_load + load)/2;
            str = ("Time: ").to_owned() + &Utc::now().to_string() + &(", Reqestes/sec: ").to_owned() + &load.to_string() + &(", Avg Reqestes/sec: ").to_owned() + &avg_load.to_string() + &("\n").to_owned();
        }
        let _ =file.write(str.as_bytes());

        i += 1;
    }
}
