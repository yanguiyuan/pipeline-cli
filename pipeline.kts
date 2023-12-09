pipeline("dev"){
    step("go"){
        workspace("./test")
        cmd("go run main.go")
    }
}
