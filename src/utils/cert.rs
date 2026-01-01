use std::{fs, path::Path, io::BufReader};
use time::{Date, OffsetDateTime, Duration,PrimitiveDateTime, Time};
use rcgen::{BasicConstraints, KeyUsagePurpose, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose,DnType, SanType, IsCa, KeyPair, Issuer};
use rustls_pemfile::{certs, pkcs8_private_keys};
use pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
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
pub fn generate_ca_cert() -> Result<(String, String), Box<dyn Error> >{
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


pub fn generate_server_cert(target_host: &str) -> Result<(String, String), Box<dyn Error>>{
    let pem_dir = "./output/";
    let cert_path = pem_dir.to_string() + "cert.pem";
    let pri_key_path = pem_dir.to_string() + "prikey.pem";

    let ca_cert = fs::read_to_string(cert_path).unwrap();
    let ca_key = fs::read_to_string(pri_key_path).unwrap();
    
    let ca_kp = KeyPair::from_pem(&ca_key).unwrap();
    let ca = Issuer::from_ca_cert_pem(&ca_cert, ca_kp).unwrap();

    let server_key = KeyPair::generate().unwrap();
    // let server_params = CertificateParams {
    //     use_authority_key_identifier_extension: true,
    //     ..CertificateParams::default()
    // };

    let mut server_params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, target_host);
    dn.push(DnType::OrganizationName, "B1ackH0rse");
    dn.push(DnType::OrganizationalUnitName, "XTransfer-Sec-Group");
    server_params.distinguished_name = dn;

    let mut san_list = Vec::new();
    san_list.push(SanType::DnsName(target_host.try_into()?));
    // 除了加入原始目标域名，也会将它的泛域名加进去
    if let Some((_,domain_suffix)) = target_host.split_once("."){
        san_list.push(SanType::DnsName(format!("*.{}",domain_suffix).try_into()?));
    }
    server_params.subject_alt_names = san_list;

    server_params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    // 1.4 扩展密钥用法（指定服务器认证）
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    // 短期有效， 1h
    let now = OffsetDateTime::now_utc();
    let expire_time = now.checked_add(Duration::hours(1)).unwrap();
    server_params.not_before = now;
    server_params.not_after = expire_time;

    let server_cert = server_params.signed_by(&server_key, &ca).unwrap();

    return Ok((server_cert.pem(), server_key.serialize_pem()))
}


// fn load_ca_and_sign_cert() {
//     let ca_cert = r#"-----BEGIN CERTIFICATE-----
// MIIFDTCCAvWgAwIBAgIUVuDfDt/BUVfObGOHsM+L5/qPZfIwDQYJKoZIhvcNAQEL
// L2Fl4tgIQ5tyaTpu9gpJZvPU0VZ/j+1Jdk1c9PJ6xhCjof4nzI9YsLbI8lPtu8K/
// Ng==
// -----END CERTIFICATE-----"#;

//     let ca_key = r#"-----BEGIN PRIVATE KEY-----
// MIIJQQIBADANBgkqhkiG9w0BAQEFAASCCSswggknAgEAAoICAQCl8mbL+2cHpPcn
// JiY98T5oN1X0C/qAXxJfSvklbru9fipwGt3dho5Tm6Ee3cYf+plnk4WZhSnqyef4
// PITGdT9dgN88nHPCle0B1+OY+OZ5
// -----END PRIVATE KEY-----"#;

//     let ca_kp = KeyPair::from_pem(ca_key).unwrap();
//     let ca = Issuer::from_ca_cert_pem(ca_cert, ca_kp).unwrap();
//     let ca_ski = vec![
//         0x97, 0xD4, 0x76, 0xA1, 0x9B, 0x1A, 0x71, 0x35, 0x2A, 0xC7, 0xF4, 0xA1, 0x84, 0x12,
//         0x56, 0x06, 0xBA, 0x5D, 0x61, 0x84,
//     ];

//     let ca_cert_der = CertificateDer::from_pem_slice(ca_cert.as_bytes()).unwrap();
//     let (_, x509_ca) = x509_parser::parse_x509_certificate(ca_cert_der.as_ref()).unwrap();

//     let ee_key = KeyPair::generate().unwrap();
//     let ee_params = CertificateParams {
//         use_authority_key_identifier_extension: true,
//         ..CertificateParams::default()
//     };
//     let ee_cert = ee_params.signed_by(&ee_key, &ca).unwrap();

//     let (_, x509_ee) = x509_parser::parse_x509_certificate(ee_cert.der()).unwrap();

// }
