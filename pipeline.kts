let c="ls"
pipeline("dev"){
    step("web"){
        cmd(c)
    }
}