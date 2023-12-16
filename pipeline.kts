pipeline("dev"){
    step("go"){
        workspace("./test")
        copy("cmd/main.go","t/main.go")
        move("t/main.go","cmd/main.go")
    }
}