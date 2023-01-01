# What is this?
This is a simple Server and Client made in Rust which uses UDP. If you want to run, make sure to have [Rust](https://www.rust-lang.org/tools/install) installed first.
* Important! You will have to manuallly update the ip addresses in both the client and server.
* There is a report that contains a much more detailed explanation of the project.

## Credits:
This was a university project, it was done with a group of three (me included!). Thanks for Ahmed Shaaban and Nashaat <3.


# Server(s):
There are two variations for the server, one does load balancing using round robin and the other uses a threshold algorithm.

## To Run:
    $cargo Run <arg1> <arg2>

The Server takes in three arguments:
* arg1: is the server id (please make sure to start from 0).
* arg2: is the fileName that will store logs about the server.


# Client(s):
The Client is a process which spawns 503 threads, 500 simulating different client applications, 2 threads for middleware and a thread for logging info.

## To Run:
    $cargo run <arg1> <arg2> <arg3>

The Client takes in three arguments:
* arg1: is the port the middleware will use to send messages/requests from.
* arg2: is the port the middleware will use to recieve and send messages/announcments from and to other middlewares.
* arg3: is the fileName that will store logs about the client.

