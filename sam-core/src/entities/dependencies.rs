use crate::entities::choices::Choice;
use crate::entities::commands::Command;
use crate::entities::identifiers::Identifier;
use crate::entities::processes::ShellCommand;
use regex::Regex;
use std::collections::HashMap;
use std::error;
use thiserror::Error;

pub trait Dependencies: Command {
    fn substitute_for_choices(
        &self,
        choices: &HashMap<Identifier, Choice>,
    ) -> Result<String, ErrorsResolver> {
        let mut command = self.command().to_string();
        for dep in self.dependencies() {
            if let Some(chce) = choices.get(&dep) {
                command = substitute_choice(&command, &dep, chce.value());
            } else {
                return Err(ErrorsResolver::NoChoiceWasAvailable(dep));
            }
        }
        Ok(command)
    }

    fn substitute_for_choices_partial(&self, choices: &HashMap<Identifier, Choice>) -> String {
        let mut command = self.command().to_string();
        for dep in self.dependencies() {
            if let Some(chce) = choices.get(&dep) {
                command = substitute_choice(&command, &dep, chce.value());
            }
        }
        command
    }
}

fn substitute_choice(origin: &str, dependency: &Identifier, choice: &str) -> String {
    let re_fmt = format!(r#"(?P<var>\{{\{{ ?{} ?\}}\}})"#, dependency.name());
    let re2_fmt = format!(
        r#"(?P<var>\{{\{{ ?{}::{} ?\}}\}})"#,
        dependency.namespace.clone().unwrap_or_default(),
        dependency.name()
    );
    let re: Regex = Regex::new(re_fmt.as_str()).unwrap();
    let re2: Regex = Regex::new(re2_fmt.as_str()).unwrap();
    let tmp = re.replace(origin, choice).to_string();
    re2.replace(&tmp, choice).to_string()
}

#[derive(Debug)]
pub struct ExecutionSequence<'repository> {
    inner: Vec<&'repository Identifier>,
}

impl<'repository> ExecutionSequence<'repository> {
    pub fn new(inner: Vec<&'repository Identifier>) -> Self {
        ExecutionSequence { inner }
    }
    pub fn identifiers(&'repository self) -> Vec<Identifier> {
        let mut rep: Vec<Identifier> = Vec::with_capacity(self.inner.len());
        for e in self.inner.clone() {
            rep.push(e.clone());
        }
        rep
    }

    pub fn as_slice(&'repository self) -> &[&'repository Identifier] {
        self.inner.as_slice()
    }
}

impl<'repository> AsRef<[&'repository Identifier]> for ExecutionSequence<'repository> {
    fn as_ref(&self) -> &[&'repository Identifier] {
        self.inner.as_slice()
    }
}

pub trait Resolver {
    fn resolve_input(&self, var: Identifier, prompt: &str) -> Result<Choice, ErrorsResolver>;
    // TODO make cmd a string
    fn resolve_dynamic<CMD>(&self, var: Identifier, cmd: CMD) -> Result<Choice, ErrorsResolver>
    where
        CMD: Into<ShellCommand<String>>;
    fn resolve_static(
        &self,
        var: Identifier,
        choices: impl Iterator<Item = Choice>,
    ) -> Result<Choice, ErrorsResolver>;
    fn select_identifier(
        &self,
        identifiers: &[Identifier],
        descriptions: Option<&[&str]>,
        prmpt: &str,
    ) -> Result<Identifier, ErrorsResolver>;
}

#[derive(Debug, Error)]
pub enum ErrorsResolver {
    #[error("no choice is available for var {0}")]
    NoChoiceWasAvailable(Identifier),
    #[error("an error happened when gathering choices for identifier {0}\n-> {1}")]
    DynamicResolveFailure(Identifier, Box<dyn error::Error>),
    #[error(
        "gathering choices for {0} failed because the command\n   {}{}{1}{} \n   returned empty content on stdout. stderr content was \n {2}", termion::color::Fg(termion::color::Cyan), termion::style::Bold, termion::style::Reset
    )]
    DynamicResolveEmpty(Identifier, String, String),
    #[error("no choice was selected for var {0}")]
    NoChoiceWasSelected(Identifier),
    #[error("no input for for var {0} because {1}")]
    NoInputWasProvided(Identifier, String),
    #[error("selection empty")]
    IdentifierSelectionEmpty(),
    #[error("selection invalid.")]
    IdentifierSelectionInvalid(Box<dyn error::Error>),
}

pub mod mocks {

    use super::{ErrorsResolver, Resolver};
    use crate::entities::choices::Choice;
    use crate::entities::identifiers::Identifier;
    use crate::entities::processes::ShellCommand;
    use std::collections::HashMap;

    #[derive(Debug)]
    pub struct StaticResolver {
        dynamic_res: HashMap<String, Choice>,
        static_res: HashMap<Identifier, Choice>,
        identifier_to_select: Option<Identifier>,
    }
    impl StaticResolver {
        pub fn new(
            dynamic_res: HashMap<String, Choice>,
            static_res: HashMap<Identifier, Choice>,
            identifier_to_select: Option<Identifier>,
        ) -> Self {
            StaticResolver {
                dynamic_res,
                static_res,
                identifier_to_select,
            }
        }
    }
    impl Resolver for StaticResolver {
        fn resolve_input(&self, var: Identifier, _: &str) -> Result<Choice, ErrorsResolver> {
            self.static_res
                .get(&var)
                .map(|e| e.to_owned())
                .ok_or(ErrorsResolver::NoChoiceWasAvailable(var))
        }

        fn resolve_dynamic<CMD>(&self, var: Identifier, cmd: CMD) -> Result<Choice, ErrorsResolver>
        where
            CMD: Into<ShellCommand<String>>,
        {
            let sh_cmd = Into::<ShellCommand<String>>::into(cmd);
            let query = sh_cmd.value();
            self.dynamic_res
                .get(query)
                .map(|e| e.to_owned())
                .ok_or(ErrorsResolver::NoChoiceWasAvailable(var))
        }
        fn resolve_static(
            &self,
            var: Identifier,
            _cmd: impl Iterator<Item = Choice>,
        ) -> Result<Choice, ErrorsResolver> {
            self.static_res
                .get(&var)
                .map(|c| c.to_owned())
                .ok_or(ErrorsResolver::NoChoiceWasSelected(var))
        }
        fn select_identifier(
            &self,
            _: &[Identifier],
            _: Option<&[&str]>,
            _: &str,
        ) -> Result<Identifier, ErrorsResolver> {
            self.identifier_to_select
                .clone()
                .ok_or(ErrorsResolver::IdentifierSelectionEmpty())
        }
    }
}
