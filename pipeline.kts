
import pipe;
pipeline("test"){
    step("t1"){
        env("Hello","123456")
        cmd("echo $Hello")
    }
}
