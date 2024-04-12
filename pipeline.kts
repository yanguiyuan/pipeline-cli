/*
class Pair{
    one:Any
    two:Any
}

fun Pair.first():Any{
    return this.one
}

val p=Pair{
    one:123,
    two:456
}
let c="123"
val a=p.first()
println(a)*/
import pipe
pipeline("test"){
    step("t1"){
        cmd("ls")
    }
}
