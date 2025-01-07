use secp256k1::ecdsa::Signature;
use secp256k1::{generate_keypair, rand, Message};
use secp256k1::hashes::{sha256, Hash};


fn as_message(value: &[u8]) -> Message {
    let digest = sha256::Hash::hash(value);
    Message::from_digest(digest.to_byte_array())
}

pub(crate) fn get_signature(value: &[u8], secret_key: &secp256k1::SecretKey) -> Signature {
    secret_key.sign_ecdsa(as_message(value))
}


pub(crate) fn check_signature(value: &[u8], signature: Signature, public_key: &secp256k1::PublicKey) -> bool {
    let message = as_message(value);
    signature.verify(&message, public_key).is_ok()
}


pub(crate) fn generate_keys() -> (secp256k1::SecretKey, secp256k1::PublicKey) {
    let (secret_key, public_key) = generate_keypair(&mut rand::thread_rng());
    return (secret_key, public_key)
}