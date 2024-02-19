#![allow(non_snake_case)]
fn main() {
    let len: usize = 56; // bounded by 1024 // MODIFY THIS!
    let x = {
        let mut x = [9; 1024]; // 9 is placeholder for out-of-bounds
        for i in 0..len {
            x[i] = 3; // 3 is placeholder for valid array value
        }
        x
    };
    let llen: [u8; 8] = len.to_le_bytes();
    let mut bytes_read = [0; 18]; // init with 0
    let mut b_len = [64; 18]; // init with 64
    let mut b = [[0; 64]; 18]; // init irrelevant
    let mut state = [5; 18]; // init with 5 as placeholder for sha256 initial state values
    
    
    for i in 1..18 {
        bytes_read[i] = if (len - bytes_read[i-1]) < 64 { len } else { bytes_read[i-1]+64 }
    }
    for i in 1..18 {
        b_len[i] = bytes_read[i] - bytes_read[i-1];
    }
    for i in 1..18 {
        for j in 0..64 {
            let B = if 56<=j && j<=63 && b_len[i]<56 { llen[j-56] } else { 0 };
            let A = if j == b_len[i] && b_len[i-1] == 64 { 128 } else { B };
            b[i][j] = if j < b_len[i] { x[bytes_read[i-1]+j] } else { A };
        }
    }
    for i in 1..18 {
        let SHA256 = state[i-1] + 1;
        state[i] = if b_len[i] + b_len[i-1]<=55 { state[i-1] } else { SHA256 }; // optimised from condition (b_len[i]==0 && b_len[i-1]<=55)
    }
    println!("bytes_read: {bytes_read:?}");
    println!("b_len:      {b_len:3?}");
    for i in 1..18 {
        print!("block {i}: {:?}", b[i]); println!(" state: {}", state[i]);
    }
    println!("len {len}: {:170}{llen:?}", " ");
}


