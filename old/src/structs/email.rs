use sailfish::TemplateOnce;

use super::RegisterUserDto;

#[derive(TemplateOnce)]
#[template(path = "verify_email.stpl")]
pub struct VerifyEmail<'a> {
    pub verification_link: &'a str,
}

#[derive(TemplateOnce)]
#[template(path = "email_verified.stpl")]
pub struct EmailVerified {}

#[derive(TemplateOnce)]
#[template(path = "reset_password.stpl")]
pub struct ResetPassword<'a> {
    pub reset_password_link: &'a str,
}

#[derive(TemplateOnce)]
#[template(path = "password_changed.stpl")]
pub struct PasswordChanged {}

#[derive(Debug)]
pub struct EmailUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
}

impl<'a> From<&'a RegisterUserDto> for EmailUser<'a> {
    fn from(user: &'a RegisterUserDto) -> Self {
        Self {
            username: &user.username,
            email: &user.email,
        }
    }
}
