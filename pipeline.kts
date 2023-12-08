pipeline("dev"){
    step("web"){
        workspace("./web")
        cmd("yarn dev")
    }
    step("tailwind"){
        workspace("./web")
        cmd("npx tailwindcss -i./src/style.css -o./src/output.css --watch")
    }
}
pipeline("hz"){
    step("new"){
         workspace("./")
         cmd("hz new -module github.com/yanguiyuan/cloudspace -idl idl/api/api.thrift")
         cmd("go mod tidy")
    }
    step("clean"){
        workspace("./")
        cmd("Remove-Item -Path ./biz -Recurse ")
        cmd("Remove-Item -Path ./script -Recurse ")
        cmd("del ./.hz")
        cmd("del ./build.sh")
        cmd("del ./router.go")
        cmd("del ./router_gen.go")
        cmd("del ./main.go")
    }
}