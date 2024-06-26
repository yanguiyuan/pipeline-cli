一个rust写的windows平台的构建工具，用于快捷执行一系列脚本。该命令行工具的想法来源是pipelight,但是pipelight不支持windows，所以写了这个工具。
### 安装
使用cargo进行安装
```powershell
cargo install pipeline-cli
```

### 内置模块
#### 1. std标准库，无需导入

- cmd(command:String) 调用sh（linux）或者powershell执行一条命令
- env(key:String,value:String) 设置当前任务的环境变量
- println(..a:Any) 输入任意值，将其打印到控制台,带换行。（注意：pipeline任务运行的时候，其日志输出会覆盖println的内容）
- print(..a:Any)
- workspace(path:String) 切换当前命令的工作空间，影响cmd，movefile,replace函数中路径的书写
- move(source_path:String,target_path:String) 将一个文件从source_path移动到target_path处，如果target_path路径不存在会尝试创建一系列文件夹
- replace(file_path:String,regex:String,replace_content:String) 通过正则将file_path处的文件中的内容替换成replace_content
- copy(source_path:String,target_path:String) 将一个文件从source_path复制到target_path处,如果target_path路径不存在会尝试创建一系列文件夹
- readInt(hint:String),
- readString(hint:String),
- readFloat(hint:String),
- readLine(hint:String),

#### 2.pipe 任务模块
使用`import pipe`导入

- pipeline(pipeline_name:String,closure:Closure):包裹一组由step和parallel组成的任务
- step(name:String,closure:Closure) 一个普通的任务，会阻塞后面的任务执行
- parallel(name:String,closure:Closure) 一个并行的任务，不会阻塞后面的任务执行

#### 3.math 数学库
- max(..a:Int|Float) 返回一串Int或者Float数中的最大值
- randomInt() 生成一个随机的Int值
- randomInt(n:Int) 生成一个随机的Int值，范围在0~n之间
- randomInt(start:Int,end:Int)生成一个随机的Int值，范围在start~end之间

#### 4.layout 项目布局生成库
- layout(name:String) 包裹一组文件
- template(targetPath:String,templatePath:String,fn:Closure) 表示的是一个文件，使用templatePath处的模板生成一个targetPath文件,fn中会有一个隐藏的ctx变量用于调用set函数
- set(ctx:Map,key:String,value:String) 设置一个映射，用于使用value替换模版文件中捕获的${key}
- folder(path:String) 创建一个目录
### 语法
1. 注释
```
//行注释
/* 注释段 */
/*
注释段
*/
```
2. 函数定义
```
fn add(a:Int,b:Int){
    return a+b
}
```
3. 函数调用
```
print(add(12,5))
let a=add(12,5)
a.print()//如果参数是函数的第一个参数，可以通过.函数的方式进行调用。
```
4. 条件判断

```
let a=true
if a{
    println("Hello")
}

```
5. 声明变量

let,var,val目前是等价的,val和var目的是使用kotlin script 的代码提示
```
let a=1
let a=1.25
let a="hello"
var a=true
val b=a
```

6.循环
```
a=3
while a>1{
    println(a)
    a=a-1
}

```

7. 算术表达式

目前仅支持+,-,*,/,%,>,<,==等二元表达式
### 将其作为Rust程序的内嵌脚本使用

```
 let ast=PipelineEngine::default()
                .eval_stmt_blocks("a=[1,2,3,4];println(a[1]);a[0]=10;println(a)").await.unwrap();

```

8.模块导入

默认会去用户目录的.pipeline/package下寻找，以下示例会去寻找~/.pipeline/package/math.kts文件，如果没找到，默认去当前目录下找math.kts
```kotlin
import math
```
### Examples
#### 使用layout定义并生成项目结构
假设我们需要再一个目录下生成项目相关文件，其结构如下：

    internal/

    pkg/

    main.go

    go.mod

则我们需要确保~/.pipeline/layout/go/layout.kts文件中内容如下：

```kts
import layout
val projectName = readString("请输入项目名:");
layout("go"){
    template("main.go","main.go.tpl")
    template("go.mod","go.mod.tpl"){
        ctx.set("projectName",projectName)
    }
    folder("internal")
    folder("pkg")
}

```
同目录下的main.go.tpl内容如下：
```go
package main

import "fmt"

func main(){
    fmt.Println("Hello,World!")
}

```
同目录下的go.mod.tpl内容如下
```go
module ${projectName}

go 1.21
```

最后我们创建一个demo目录，切换到demo目录下，使用一下命令：
```bash
pipeline layout go
```
它会提示你输入项目名，输入完毕后会生成对应的结构


#### pipeline 配置多个任务
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
    step("go"){
        workspace("./test")
        copy("cmd/main.go","t/main.go")
        move("t/main.go","cmd/main.go")
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
1.执行Pipeline dev下的Step web相关命令：

```powershell
pipeline run dev.web
或者运行dev下的所有step
pipeline run dev
```
![img.png](assets/img.png)

2.列出所有的任务

```powershell
pipeline list
```
3.初始化项目使用模板

```powershell
pipeline init -t <template_name>
//或者
pipeline init --template <template_name>
//例如
pipeline init -t hertz-vue
```
4.列出所有可用的模版（模版存放在用户目录的.pipeline目录下，可能需要你手动创建目录）

```powershell
pipeline template
```
5.将当前项目中的pipeline.kts保存为模版

```powershell
pipeline template -a/--add <template_name>
```
6.移除指定模版

```powershell
pipeline template -r/--remove <template_name>
```