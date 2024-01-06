pipeline("dev"){
    step("go"){
        workspace("./test")
        copy("x","s")
    }
}