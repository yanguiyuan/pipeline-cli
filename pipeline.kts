/*
struct Person{
    name:String
    age:Int
}
*/
let p=Person{
    name:"张三",
    age:11
}
p.age=18
println(p.name)
p.name="李四"
println(p.name)
println(p.type().len().type())
