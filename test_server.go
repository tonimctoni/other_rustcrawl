package main;

import "net/http"
import "time"
import "fmt"
import "sync/atomic"
import "os"
import "os/signal"
import "syscall"
import "flag"
import "io/ioutil"

func main() {
    filename:=flag.String("file", "", "Name of file to send as http response.")
    content_type:=flag.String("type", "text/plain", "Content-Type to be set in header of response.")
    addr:=flag.String("addr", ":8080", "Addr to listen to.")
    flag.Parse()
    message:=func() []byte{
        if *filename==""{
            return []byte("Hello there")
        }

        content,err:=ioutil.ReadFile(*filename)
        if err!=nil{
            fmt.Println("ioutil.ReadFile(...):", err)
            panic("ioutil.ReadFile(...)")
        }

        return content
    }()


    mux:=http.NewServeMux()
    server:=&http.Server{
        Addr: *addr,
        ReadTimeout: 5*time.Second,
        WriteTimeout: 5*time.Second,
        IdleTimeout: 5*time.Second,
        Handler: mux,
    }
    counter:=new(int64)
    error_counter:=new(int64)


    mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request){
        w.Header().Set("Content-Type", *content_type)
        w.WriteHeader(http.StatusOK)
        _,err:=w.Write(message)
        if err!=nil{
            atomic.AddInt64(error_counter, 1)
        } else{
            atomic.AddInt64(counter, 1)
        }
    })


    server_error:=make(chan error)
    go func(){
        err:=server.ListenAndServe()
        server_error <- err
    }()


    sigterm:=make(chan os.Signal)
    signal.Notify(sigterm, os.Interrupt, syscall.SIGTERM)


    outer_for: for{
        select{
        case <-time.After(time.Second):
            num_requests:=atomic.SwapInt64(counter, 0)
            num_errors:=atomic.SwapInt64(error_counter, 0)
            fmt.Println(num_requests)
            if num_errors!=0{
                fmt.Println("Errors:", num_errors)
            }
        case sig:=<-sigterm:
            fmt.Println("Sigterm received:", sig)
            err:=server.Close()
            fmt.Println("Server.Close():", err)
        case err:=<-server_error:
            fmt.Println("Server.ListenAndServe():", err)
            break outer_for
        }
    }
}