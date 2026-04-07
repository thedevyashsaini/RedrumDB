pub mod core;
pub mod lists;
pub mod pubsub;
pub mod strings;

pub const PUBSUB_HANDLER: &[&[u8]] = &[
    b"subscribe",
    b"unsubscribe",
    b"psubscribe",
    b"punsubscribe",
    b"ping",
    b"quit",
];

#[macro_export]
macro_rules! command_handler {
    ($name:ident, $args:ident, $ctx:ident, $body:block) => {
        pub fn $name(
            $args: &[&[u8]],
            $ctx: &mut Context,
        ) -> Result<Vec<u8>, Vec<u8>> {

            if *$ctx.is_pubsub
                && !crate::commands::handlers::PUBSUB_HANDLER.contains(&stringify!($name).as_bytes())
            {
                let mut res = Vec::with_capacity(122 + stringify!($name).len());

                res.extend_from_slice(b"-ERR Can't execute '");
                res.extend_from_slice(stringify!($name).as_bytes());
                res.extend_from_slice(
                    b"': only (P|S)SUBSCRIBE / (P|S)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context\r\n"
                );

                return Err(res);
            }

            $body
        }
    };
}