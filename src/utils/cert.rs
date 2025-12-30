use std::{fs, path::Path, io::BufReader};
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time};
use rcgen::{BasicConstraints, KeyUsagePurpose, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair};
use rustls_pemfile::{certs, pkcs8_private_keys, read_one, read_all};
use pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer};
use std::io;
use std::error::Error;


/// 参考文档：https://blog.csdn.net/yuan__once/article/details/137635953
/// 加载已经生成的证书进程序
pub fn load_cert() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), &'static str>{
    let pem_dir = "./output/";
    let cert_path = pem_dir.to_string() + "cert.pem";
    let pri_key_path = pem_dir.to_string() + "prikey.pem";

    if Path::new(&cert_path).exists() && Path::new(&pri_key_path).exists() {
        let cert_fs= fs::File::open(&cert_path).unwrap();
        let key_fs = fs::File::open(&pri_key_path).unwrap();
        let mut cert_reader = BufReader::new(cert_fs);
        let mut key_reader = BufReader::new(key_fs);

        let certs = certs(&mut cert_reader)
            .filter_map(|result|{
                match result {
                    Ok(cert) => Some(cert),
                    Err(_) => None,
                }   
            }).collect();

        let keys: Vec<PrivatePkcs8KeyDer<'static>> = pkcs8_private_keys(&mut key_reader)
             .filter_map(|result|{
                match result {
                    Ok(key) => Some(key),
                    Err(_) => None,
                }   
            }).collect();
        
        let pkcs8_key = keys.into_iter().next().ok_or("No private key found")?;
        let key = PrivateKeyDer::Pkcs8(pkcs8_key);
        Ok((certs, key))
    }
    else{
        return Err("either cert.pem or it's private-key not exists, Generate one please.")
    }
}

/// 在没有证书的情况下使用命令行参数可以生成一个证书，客户端需安装它并选择信任它。
pub fn generate_cert() -> Result<(String, String), Box<dyn Error> >{
    let mut ca_params = CertificateParams::default();

    // add Domain Name
    let mut dn = DistinguishedName::default();
    dn.push(DnType::CommonName, "XT-Sec");
    dn.push(DnType::OrganizationName, "B1ackH0rse");
    dn.push(DnType::OrganizationalUnitName, "XTransfer-Sec-Group");
    ca_params.distinguished_name = dn;

    // set CA properties
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

    // set purpose explanation
    ca_params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature, // 用于数字签名
        KeyUsagePurpose::KeyCertSign,  // 用于签发子证书
        KeyUsagePurpose::CrlSign,      // 允许吊销列表签名
    ];

    // set expire time
    let now = OffsetDateTime::now_utc();
    let target = Date::from_calendar_date(now.year()+3, now.month(), now.day());
    let primitive_dt = PrimitiveDateTime::new(target.expect("invalid datetime"), Time::MIDNIGHT);

    ca_params.not_before = now;
    ca_params.not_after = primitive_dt.assume_utc();


    // get the key pair
    let key_pair = KeyPair::generate().unwrap();
    // get certificate 
    let cert = ca_params.self_signed(&key_pair).unwrap();


    // get string tuple
    let cert_str = cert.pem();
    let kp_str = key_pair.serialize_pem();
    Ok((cert_str, kp_str))
    // Ok((cert_der.to_vec(), kp_der.to_vec()))

}

