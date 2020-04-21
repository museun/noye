#[derive(Clone, Debug)]
pub struct Message {
    pub sender: String,
    pub channel: String,
    pub data: String,
}
