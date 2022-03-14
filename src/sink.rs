use crate::frame::{Frame, ReqBuf};

#[derive(Clone, Debug)]
pub struct Sink {}


impl Sink {
    pub fn encode(&self, id: u64, ok: u8, data: Vec<u8>) -> Vec<Frame> {
        let max_frame_len = crate::get_frame_len();
        if data.len() > max_frame_len as usize {
            let mut frames_len = data.len() / max_frame_len as usize;
            if (frames_len * max_frame_len as usize) < data.len() {
                frames_len += 1;
            }
            let mut datas = Vec::with_capacity(frames_len);
            for idx in 0..frames_len {
                let mut max = ((idx + 1) * max_frame_len as usize);
                if max > data.len() {
                    max = data.len();
                }
                datas.push(Frame {
                    id,
                    ok,
                    data: data[(idx * max_frame_len as usize)..max].to_vec(),
                });
            }
            datas
        } else {
            vec![Frame {
                id,
                ok,
                data,
            }]
        }
    }

    pub fn decode(&self, frames: Vec<Frame>) -> Option<Frame> {
        let mut result = None;
        for x in frames {
            if result.is_none() {
                result = Some(Frame {
                    id: x.id,
                    ok: x.ok,
                    data: x.get_payload().to_vec(),
                });
                continue;
            } else {
                result.as_mut().unwrap().data.extend(x.get_payload().to_vec());
            }
        }
        result
    }
}

#[cfg(test)]
mod test {
    use crate::sink::Sink;

    #[test]
    fn test_encode() {
        crate::set_frame_len(2);
        let s = Sink {};
        let v = s.encode(1, 1, vec![1, 2, 3]);
        println!("{:?}", v);
    }
}