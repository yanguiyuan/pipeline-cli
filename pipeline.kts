pipeline("dev"){
    step("go"){
        workspace("./test")
        movefile("a.txt","hello/a.txt")
    }
    parallel("echo"){
         workspace("./test")
         cmd("go run main.go")
    }
}


pipeline("prod"){
    step("go"){
        workspace("./test")
        cmd("go run main.go")
    }
}