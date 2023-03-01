use std::{
    collections::VecDeque,
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
};

//缓冲区长度
const BUFFER_LENGTH: usize = 1024 * 1024 * 20;

fn main() {
    //解析命令行参数，获得port和target path
    //cargo run --release -- --port 8000 {target dir}
    let mut args = env::args().skip(1).collect::<VecDeque<String>>();
    let port_index = args
        .binary_search(&"--port".to_string())
        .expect("port not found!");
    args.remove(port_index);
    let port = args.remove(port_index).expect("port not exists!");
    let target_dir = args.get(0).expect("target dir not exists!");

    //监听port，等待发送端发起连接
    let listener =
        TcpListener::bind(format!("192.168.0.103:{port}")).expect("ip port bind failure!");
    let (stream, _) = listener.accept().expect("tcp connection failure!");
    let mut stream = BufReader::with_capacity(BUFFER_LENGTH, stream);

    parse_and_write(target_dir, &mut stream);
}

//解析接收的数据
fn parse_and_write(target_dir: &str, stream: &mut BufReader<TcpStream>) {
    let mut buffer = vec![0_u8; BUFFER_LENGTH];
    loop {
        //读出描述数据，无法读出描述数据，说明文件传输完成
        let mut description = Vec::new();
        stream
            .read_until(b'\0', &mut description)
            .expect("read data failure!");
        if description.is_empty() {
            break;
        }
        description.pop();
        let description = String::from_utf8(description).expect("is not utf8!");

        //解析type_id和relative path
        let mut description = description.split(':');
        let type_id = description.next().expect("type id not found!");
        let relative_path = description.next().expect("path not found!");

        let mut path = String::from(target_dir);
        //根据所在的操作系统转换目录分隔符
        if env::consts::OS == "windows" {
            path.push('\\');
            path.push_str(&relative_path.replace('/', "\\"));
        } else {
            path.push('/');
            path.push_str(&relative_path.replace('\\', "/"));
        }
        let path = Path::new(&path);

        println!("{}", path.display());

        match type_id {
            //是文件夹，创建文件夹
            "folder" => fs::create_dir(path).expect("create dir failure!"),
            //是文件，创建文件写入数据
            "file" => {
                //解析要接收的文件大小
                let length = description
                    .next()
                    .expect("length not found!")
                    .parse::<usize>()
                    .expect("length parse failure!");
                
                //计算能装满多少个缓冲区和剩余长度
                let while_count = length / BUFFER_LENGTH;
                let last_length = length % BUFFER_LENGTH;

                //创建文件并向文件中写入数据
                let mut file = File::create(path).expect("create file failure!");
                for _ in 0..while_count {
                    stream.read_exact(&mut buffer).expect("read data failure!");
                    file.write_all(&buffer).expect("file write failure!");
                }
                stream
                    .read_exact(&mut buffer[..last_length])
                    .expect("read data filure!");
                file.write_all(&buffer[..last_length])
                    .expect("file write failure!");
            }
            _ => panic!("type id error!"),
        }
    }
}
