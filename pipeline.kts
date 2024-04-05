import pipe

pipeline("dev"){
	step("run"){
		cmd("echo hello")
		//cmd("go run main.go")
	}
}
/*
pipeline("dev"){
    step("run"){
        cmd("echo hello")
    }
}
*/