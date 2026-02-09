#[rocket::launch]
fn rocket() -> _ {
    local_agent_chat::rocket()
}
