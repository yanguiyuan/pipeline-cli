let a=[1,2]
a.append(a.clone())
println(a)

for it in a{
    println(it)
    if it.type()=="Array"{
        for i in it{
            println(i)
        }
    }
}

