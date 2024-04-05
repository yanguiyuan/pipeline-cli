import layout

layout("go"){
    template("tests/main.go","main.tpl"){
        ctx.set("content","Hello,World")
    }
}