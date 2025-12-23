use std::{path::Path, fs};
use rcgen::{Certificate, KeyPair, CertificateParams, IsCa, BasicConstraints, Issuer};
use std::error::Error;


/// 参考文档：https://blog.csdn.net/yuan__once/article/details/137635953
/// 加载已经生成的证书进程序
// pub fn load_cert() -> Result<(Issuer<'_>, KeyPair), &'static str>{
//     if Path::new("ca.pem").exists() && Path::new("prikey.pem").exists() {
//         let ca_str = fs::read_to_string("ca.pem").unwrap();
//         let key_str = fs::read_to_string("prikey.pem").unwrap();
//         // TODO: 不兼容，需检索正确的加载方式。
//         let key = KeyPair::from_pem(&key_str).unwrap();
//         // 官方用法：https://github.com/rustls/rcgen/blob/b250fa36a553a7d090d36ad70886c97191065581/rcgen/src/certificate.rs#L1362
//         let ca = Issuer::from_ca_cert_pem(&ca_str,ca_kp).unwrap();
//         Ok((ca, key))
//     }
//     else{
//         return Err("either ca.pem or it's private-key not exists, Generate one please.")
//     }
// }

/// 在没有证书的情况下使用命令行参数可以生成一个证书，客户端需安装它并选择信任它。
pub fn generate_cert() -> Result<(String, String), Box<dyn Error> >{
    
    // default algorithm is PKCS_ECDSA_P256_SHA256
    let kp = KeyPair::generate();
    if let Ok(key_pair) = kp {
        let pem = key_pair.public_key_pem();
        let kp_pem = key_pair.serialize_pem();
        Ok((pem, kp_pem))
    }else{
        panic!("Error in creating certifications!");
    }
}

