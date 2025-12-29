use std::{fs, path::Path};
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use rcgen::{BasicConstraints, KeyUsagePurpose, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair};
use rustls_pemfile::{certs, pkcs8_private_keys}; 
use std::error::Error;


/// 参考文档：https://blog.csdn.net/yuan__once/article/details/137635953
/// 加载已经生成的证书进程序
// pub fn load_cert() -> Result<(Issuer<'_>, KeyPair), &'static str>{
//     let pem_dir = "./output/";
//     let ca_path = pem_dir.to_string() + "cert.pem";
//     let pri_key_path = pem_dir.to_string() + "prikey.pem";

//     if Path::new(&ca_path).exists() && Path::new(&pri_key_path).exists() {
//         let ca_str: String = fs::read_to_string(&ca_path).unwrap();
//         let key_str: String = fs::read_to_string(&pri_key_path).unwrap();
//         // TODO: 不兼容，需检索正确的加载方式。
//         // let cert = Certificate::from_pem(&ca_str).unwrap();
//         // let key: KeyPair = KeyPair::from_pem(&key_str).unwrap();
//         Ok((cert, key))
//     }
//     else{
//         return Err("either ca.pem or it's private-key not exists, Generate one please.")
//     }
// }

/// 在没有证书的情况下使用命令行参数可以生成一个证书，客户端需安装它并选择信任它。
pub fn generate_cert() -> Result<(Vec<u8>, Vec<u8>), Box<dyn Error> >{
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
    let cert_der = cert.der().as_ref();
    let kp_der = key_pair.serialized_der();
    Ok((cert_der.to_vec(), kp_der.to_vec()))

}

