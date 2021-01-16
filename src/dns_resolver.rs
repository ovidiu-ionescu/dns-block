use std::net::UdpSocket;
use std::thread;

fn write(n: u16, vec: &mut Vec<u8>, index: usize) {
    let be = n.to_be_bytes();
    vec[index] = be[0];
    vec[index + 1] = be[1];
}

fn read(buf: &[u8], start: usize) -> usize {
    u16::from_be_bytes([buf[start], buf[start + 1]]) as usize
} 

fn extract_name(buf: &[u8], start: usize) -> String {
    let mut res = String::new();
    let mut crt = start;
    let mut len = buf[start] as usize;
    while len > 0 {
        if len >= 192 {
            crt = u16::from_be_bytes([(len - 192) as u8, buf[crt + 1]]) as usize;
            len = buf[crt] as usize;
        }
        res.push_str(std::str::from_utf8(&buf[crt + 1 ..= crt + len]).unwrap());
        crt += len  + 1;
        len = buf[crt] as usize;
        if len != 0 {
            res.push('.');
        }
    }
    res
}

fn create_request(domain: &str, id: u16) -> Vec<u8> {
    let size = 12 + domain.len() + 2 + 4;
    let mut header: Vec<u8> = Vec::with_capacity(size);
    header.resize(size, 0);
    write(id, &mut header, 0);
    header[2] = 1; 
    header[5] = 1;
    header[size - 3] = 1;
    header[size - 1] = 1;

    unsafe {
        std::ptr::copy_nonoverlapping(domain.as_ptr(), header.as_mut_ptr().offset(12 + 1), domain.len());
    }
    header[12] = b'.';
    let mut cnt = 0;
    for c in header[12 ..=12 + domain.len()].iter_mut().rev() {
        // print!("{}", *c as char);
        if *c == b'.' {
            *c = cnt;
            cnt = 0;
        } else {
            cnt += 1;
        }
    }
    header
 }

fn extract_data(buf: &[u8], result: &mut Vec<String>) {
    //println!("Process response for {}, answers {}", extract_name(buf, 12), u16::from_be_bytes([buf[6], buf[7]]));

    // compute question length
    let question_length = compute_url_length(buf, 12) + 4;
    let answer_count = read(buf, 6);
    let mut answer_start = 12 + question_length;
    for _x in 0 .. answer_count {
        let url_length = compute_url_length(buf, answer_start);
        if 5 == read(buf, answer_start + url_length) {
            let cname = extract_name(buf, answer_start + url_length + 10);
            //println!("{}", &cname);
            result.push(cname);

        }
        answer_start += url_length + 10 + read(buf, answer_start + url_length + 8);
    }
}

fn compute_url_length(buf: &[u8], start: usize) -> usize {
    let mut len = buf[start] as usize;
    let mut crt = start;
    while len > 0 && len < 192 {
        crt += len + 1;
        len = buf[crt] as usize;
    }
    if len == 0 {
        crt + 1 - start
    } else {
        crt + 2 - start
    }
} 

pub fn resolve_domain(domains_str: &Vec<&str>, result: &mut Vec<String>) -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:6913").expect("couldn't bind to address");
    socket.connect("8.8.8.8:53").expect("connect function failed");

    let domains: Vec<String> = domains_str.iter().map(|s| String::from(*s)).collect();
    let domain_count = domains.len();
    let send_socket = socket.try_clone()?;
    let handle = thread::spawn(move || {

        let id: u16 = std::process::id() as u16;
        for domain in domains {
            let request = create_request(domain.as_str(), id);
            //id += 1;
            //fs::write("req.bin", &request)?;

            send_socket.send(&request).expect("could not send the message");
        }
    });

    // the requests have been sent, now we deal with the answers

    let mut resp = [0; 512];
    for _x in 0 .. domain_count {
        //println!("Waiting for domain {}", _x);
        let received = socket.recv(&mut resp)?;
        //fs::write("answer.bin", &resp[0 ..received])?;

        extract_data(&resp[0 ..received], result);
    }
    handle.join().unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {

/*
00000000: 10e8 8180 0001 0004 0000 0000 03 w  w w  .............www
00000010: 08 b  a x  - s  h o  p02  n l 0000 0100  .bax-shop.nl....
00000020: 01c0 0c00 0500 0100 000d 5700 1f03  w w  ..........W...ww
00000030:  w08  b a  x -  s h  o p 02 n  l09  e d  w.bax-shop.nl.ed
00000040:  g e  s u  i t  e03  n e  t00 c02d 0005  gesuite.net..-..
00000050: 0001 0000 5363 0011 05 a  1 9  5 8 01 r  ....Sc...a1958.r
00000060: 06 a  k a  m a  ic0 47c0 5800 0100 0100  .akamai.G.X.....
00000070: 0000 1300 049510097 90c0 5800 0100 0100  ....._daj.X.....
00000080: 0000 1300 049510097 90                   ....._daZ

type class TTL pointer

Question:
03www08bax-shop02nl00
0001
0001

Answer
1)
c00c link
0005 CNAME
0001 IN
00000d57 TTL 
001f 31bytes
3www8bax-shop2nl9edgesuite3net0
2)
c02d 
0005 CNAME
0001
0000 5363 TTL
0011 17bytes
05a195801r06akamaic047 (link to .net)
3)
c0 58 link to akamai
0001 A
0001 IN
00000013 TTL
0004 95.100.9790
4)
c058 link to akamai
0001 A
0001 IN
00000013 TTL
0004 95.100.97.90
*/
    const BUF: [u8; 137] = [
        0x10, 0xe8, 0x81, 0x80, 0x00, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x03, 0x77, 0x77, 0x77,
        0x08, 0x62, 0x61, 0x78, 0x2d, 0x73, 0x68, 0x6f, 0x70, 0x02, 0x6e, 0x6c, 0x00, 0x00, 0x01, 0x00,
        0x01, 0xc0, 0x0c, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x0d, 0x57, 0x00, 0x1f, 0x03, 0x77, 0x77,
        0x77, 0x08, 0x62, 0x61, 0x78, 0x2d, 0x73, 0x68, 0x6f, 0x70, 0x02, 0x6e, 0x6c, 0x09, 0x65, 0x64,
        0x67, 0x65, 0x73, 0x75, 0x69, 0x74, 0x65, 0x03, 0x6e, 0x65, 0x74, 0x00, 0xc0, 0x2d, 0x00, 0x05,
        0x00, 0x01, 0x00, 0x00, 0x53, 0x63, 0x00, 0x11, 0x05, 0x61, 0x31, 0x39, 0x35, 0x38, 0x01, 0x72,
        0x06, 0x61, 0x6b, 0x61, 0x6d, 0x61, 0x69, 0xc0, 0x47, 0xc0, 0x58, 0x00, 0x01, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x13, 0x00, 0x04, 0x5f, 0x64, 0x61, 0x6a, 0xc0, 0x58, 0x00, 0x01, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x13, 0x00, 0x04, 0x5f, 0x64, 0x61, 0x5a
    ];
    #[test]
    fn test_response_parsing() {
       assert_eq!("www.bax-shop.nl", super::extract_name(&BUF, 12));
       assert_eq!("a1958.r.akamai.net", super::extract_name(&BUF, 121));
       println!("{}", super::extract_name(&BUF, 45));
    }

    #[test]
    fn test_calculate_url_length() {
        assert_eq!(17, super::compute_url_length(&BUF, 12));
        assert_eq!(2, super::compute_url_length(&BUF, 33));
        assert_eq!(17, super::compute_url_length(&BUF, 88));
    }
}
