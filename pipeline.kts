
import pipe;
pipeline("test"){
    step("t1"){
        workspace("./test")
        env("Hello","123456")
        cmd("ls")
    }
}