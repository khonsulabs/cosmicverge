/// A public Certificate. You can distribute it freely to peers.
#[derive(Clone, Debug)]
pub struct Certificate(pub(crate) Vec<u8>);

/// A private Key.
///
/// # Safety
/// Never give this to anybody.
#[allow(missing_debug_implementations)]
pub struct PrivateKey(pub(crate) Vec<u8>);

/// Generate a self signed certificate.
#[cfg(feature = "certificate")]
pub fn generate_self_signed<S: Into<String>>(domain: S) -> (Certificate, PrivateKey) {
    #[allow(clippy::expect_used)]
    let certificate = rcgen::generate_simple_self_signed(vec![domain.into()])
        .expect("`rcgen` failed generating a self-signed certificate");

    (
        #[allow(clippy::expect_used)]
        Certificate(
            certificate
                .serialize_der()
                .expect("`rcgen` failed serializing a certificate"),
        ),
        PrivateKey(certificate.serialize_private_key_der()),
    )
}
