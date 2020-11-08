use crate::core::aliases::Alias;
use crate::core::choices::Choice;
use crate::core::namespaces::NamespaceUpdater;
use crate::core::vars::Var;
use crate::core::vars_repository::{ErrorsVarsRepository, VarsRepository};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

pub fn read_aliases_from_path(path: &'_ Path) -> Result<Vec<Alias>, ErrorsAliasRead> {
    let f = File::open(path)?;
    let buf = BufReader::new(f);
    let mut aliases = read_aliases(buf).map_err(|error| ErrorsAliasRead::AliasSerde {
        error,
        source_file: path.to_path_buf(),
    })?;

    for a in aliases.as_mut_slice() {
        NamespaceUpdater::update_from_path(a, path);
    }

    Ok(aliases)
}

fn read_aliases<T>(r: T) -> Result<Vec<Alias>, serde_yaml::Error>
where
    T: Read,
{
    serde_yaml::from_reader(r)
}

pub fn read_choices<T>(r: T) -> Result<Vec<Choice>, ErrorsChoiceRead>
where
    T: BufRead,
{
    let mut out = vec![];
    for line_r in r.lines() {
        let line = line_r?;
        let splits: Vec<&str> = line.split('\t').collect();
        let value_o = splits.get(0).map(|e| e.to_string());
        let desc = splits.get(1).map(|e| e.to_string());
        if let Some(value) = value_o {
            out.push(Choice::new(value, desc));
        }
    }
    Ok(out)
}

pub fn read_vars_repository(path: &'_ Path) -> Result<VarsRepository, ErrorsVarRead> {
    let f = File::open(path)?;
    let buf = BufReader::new(f);
    let mut vars = read_vars(buf).map_err(|e| ErrorsVarRead::VarsSerde {
        error: e,
        source_file: path.to_path_buf(),
    })?;

    for a in vars.as_mut_slice() {
        NamespaceUpdater::update_from_path(a, path);
    }

    VarsRepository::new(vars.into_iter()).map_err(|e| e.into())
}

fn read_vars<T>(r: T) -> Result<Vec<Var>, serde_yaml::Error>
where
    T: Read,
{
    serde_yaml::from_reader(r)
}

#[derive(Debug, Error)]
pub enum ErrorsAliasRead {
    #[error("parsing error for aliases file {source_file}\n-> {error}.")]
    AliasSerde {
        error: serde_yaml::Error,
        source_file: PathBuf,
    },
    #[error("got an IO error while reading file\n-> {0}")]
    AliasIO(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ErrorsVarRead {
    #[error("parsing error for vars file {source_file}\n-> {error}.")]
    VarsSerde {
        error: serde_yaml::Error,
        source_file: PathBuf,
    },
    #[error("got an IO error while reading file\n-> {0}")]
    VarIO(#[from] std::io::Error),
    #[error("initialisation failure because\n{0}")]
    VarsRepositoryInit(#[from] ErrorsVarsRepository),
}

#[derive(Debug, Error)]
pub enum ErrorsChoiceRead {
    #[error("got an IO error while reading choices\n-> {0}")]
    ChoiceIO(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::{read_aliases, read_vars};
    use crate::core::aliases::Alias;
    use crate::core::choices::Choice;
    use crate::core::vars::Var;
    use std::io::BufReader;
    use std::panic;
    #[test]
    fn test_read_vars() {
        let vars_str = "
            - desc: 'desc1'
              name: 'name1'
              choices:
              - value: 'val1'
                desc: val1 description
            - desc: 'desc2'
              name: 'name2'
              choices:
              - value: 'val2'
                desc: val2 description
              - value: 'val1'
                desc: val1 description"
            .as_bytes();

        let r = BufReader::new(vars_str);
        let vars_r = read_vars(r);
        assert!(vars_r.is_ok());
        let vars = vars_r.unwrap();
        assert_eq!(vars.len(), 2);
        let exp_choices_1 = vec![Choice::new("val1", Some("val1 description"))];
        let exp_choices_2 = vec![
            Choice::new("val2", Some("val2 description")),
            Choice::new("val1", Some("val1 description")),
        ];
        let exp_var_listing = Var::new("name1", "desc1", exp_choices_1);
        let exp_var_2 = Var::new("name2", "desc2", exp_choices_2);
        assert_eq!(vars, vec![exp_var_listing, exp_var_2]);
    }
    #[test]
    fn test_read_aliases() {
        let aliase_str = "
            - desc: 'desc1'
              name: 'name1'
              alias: 'alias1'
            - desc: 'desc2'
              name: 'name2'
              alias: 'alias2'"
            .as_bytes();
        let r = BufReader::new(aliase_str);
        let aliases_r = read_aliases(r);
        assert!(aliases_r.is_ok());
        let aliases = aliases_r.unwrap();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0], Alias::new("name1", "desc1", "alias1"));
        assert_eq!(aliases[1], Alias::new("name2", "desc2", "alias2"));

        let aliase_str = "
            - desc: 'desc1'
              alias: 'alias1'
            - desc: 'desc2'
              alias: 'alias2'"
            .as_bytes();
        let r = BufReader::new(aliase_str);
        let aliases_r = read_aliases(r);
        assert!(aliases_r.is_err());
    }
}
