一个rust写的windows平台的构建工具，用于快捷执行一系列脚本。该命令行工具的想法来源是pipelight,但是pipelight不支持windows，所以写了这个工具。

### Examples
需要在项目目录下添加一个名为pipeline.kts的文件，文件语法采用kotlin dsl语法，仅支持函数使用内建函数进行调用
一个pipeline.kts的例子:
```kotlin
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
```
执行Pipeline dev下的Step web相关命令：
```powershell
pipeline run dev.web
```
列出所有的任务
```powershell
pipeline list
```
初始化项目使用模板
```powershell
pipeline init -t <template_name>
//或者
pipeline init --template <template_name>
//例如
pipeline init -t hertz-vue
```
