//! Generate a private test CA and a distinct localhost server certificate.
use anyhow::{Result, ensure};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

pub fn certificates(directory: &Path) -> Result<()> {
    fs::create_dir_all(directory)?;
    let openssl = |args: &[&str]| -> Result<()> {
        let status = Command::new("openssl")
            .args(args)
            .current_dir(directory)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        ensure!(status.success(), "test certificate generation failed");
        Ok(())
    };
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "1",
        "-subj",
        "/CN=Postern test CA",
        "-keyout",
        "ca-key.pem",
        "-out",
        "ca.pem",
        "-addext",
        "basicConstraints=critical,CA:TRUE",
    ])?;
    openssl(&[
        "req",
        "-new",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-subj",
        "/CN=localhost",
        "-keyout",
        "server-key.pem",
        "-out",
        "server.csr",
    ])?;
    fs::write(
        directory.join("server.ext"),
        concat!(
            "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n",
            "extendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost\n"
        ),
    )?;
    openssl(&[
        "x509",
        "-req",
        "-in",
        "server.csr",
        "-CA",
        "ca.pem",
        "-CAkey",
        "ca-key.pem",
        "-CAcreateserial",
        "-days",
        "1",
        "-out",
        "server.pem",
        "-extfile",
        "server.ext",
    ])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Only this disposable test key is readable by Keycloak's container UID.
        fs::set_permissions(
            directory.join("server-key.pem"),
            fs::Permissions::from_mode(0o644),
        )?;
    }
    Ok(())
}
