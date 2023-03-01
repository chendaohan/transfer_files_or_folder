use std::{
    collections::VecDeque,
    env,
    fs::File,
    io::{Read, Write, BufWriter},
    net::TcpStream,
    path::Path,
};

//缓冲区长度
const BUFFER_LENGTH: usize = 1024 * 1024 * 20;

fn main() {
    //解析命令行参数，获得ip、port和path
    //cargo run --release -- --ip-port {receiver ip port} {dir or file}...
    let mut args = env::args().skip(1).collect::<VecDeque<String>>();
    let ip_port_index = args
        .binary_search(&String::from("--ip-port"))
        .expect("ip port not found!");
    args.remove(ip_port_index);
    let ip_port = args.remove(ip_port_index).expect("ip port not exists!");

    //发起tcp连接，创建一个缓冲区
    let stream = TcpStream::connect(ip_port).expect("tcp connection failure!");
    let mut stream = BufWriter::with_capacity(BUFFER_LENGTH, stream);
    let mut buffer = vec![0_u8; BUFFER_LENGTH];

    //遍历命令行中的path
    for path in args.into_iter() {
        let path = Path::new(&path);
        //从绝对路径中取得要传输的文件或文件夹的起始索引
        let path_parent = path.parent().expect("no parent!").to_string_lossy();
        let mut start_index = path_parent.len();
        //如果这个路径的父目录不是根目录就加 1
        match env::consts::OS {
            "windows" if start_index > 3 => start_index += 1,
            "linux" if path_parent != "/" => start_index += 1,
            _ => ()
        }
        traverse_and_send(start_index, path, &mut buffer, &mut stream);
    }

    stream.flush().expect("flush output stream!");
}

//递归遍历和发送所有文件
fn traverse_and_send(start_index: usize, path: &Path, buffer: &mut [u8], stream: &mut BufWriter<TcpStream>) {
    println!("{}", path.display());

    //路径是文件夹，发送文件夹信息，读取文件夹内路径递归调用
    //folder:{path}\0
    if path.is_dir() {
        stream
            .write_all(format!("folder:{}\0", &path.to_string_lossy()[start_index..]).as_bytes())
            .expect("write data failure!");
        let read_dir = path.read_dir().expect("read dir failure!");
        for dir_entry in read_dir {
            let dir_entry = dir_entry.unwrap();
            traverse_and_send(start_index, &dir_entry.path(), buffer, stream);
        }
    }

    //路径是文件，发送文件信息和文件内部数据
    //file:{path}:{length}\0
    if path.is_file() {
        let mut file = File::open(path).expect("open file failure!");
        let description = format!(
            "file:{}:{}\0",
            &path.to_string_lossy()[start_index..],
            file.metadata().unwrap().len()
        );
        stream
            .write_all(description.as_bytes())
            .expect("write data failure!");

        while let Ok(length) = file.read(buffer) {
            stream
                .write_all(&buffer[..length])
                .expect("write data failure!");

            if length < buffer.len() {
                break;
            }
        }
    }
}
