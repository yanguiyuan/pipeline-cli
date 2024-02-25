pipeline("dev"){
    step("go"){
        workspace("test")
        cmd("ls")
    }
}