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
class JVMObject{
    name:String
}
fun JVMObject.version(n:Int):Unit{
    println(this.name,":",n)
}
fun jvm(name:String):JVMObject{
    return JVMObject{
        name:name
    }
}
jvm("kotlin").version(17)
