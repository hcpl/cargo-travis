extern crate cargo;

use std::path::{Path};
use std::ffi::{OsString};
use cargo::core::{Workspace};
use cargo::ops::{CompileOptions, ExecEngine, ProcessEngine, CommandType, CommandPrototype};
use cargo::util::{CargoTestError};
use cargo::{CargoResult};

pub struct CoverageOptions<'a> {
    pub compile_opts: CompileOptions<'a>,
    pub merge_dir: &'a Path,
    pub merge_args: Vec<OsString> // TODO: Or &[str] ?
}

pub fn run_coverage(ws: &Workspace, options: &CoverageOptions, test_args: &[String]) -> CargoResult<Option<CargoTestError>> {
    let mut compilation = try!(cargo::ops::compile(ws, &options.compile_opts));
    compilation.tests.sort_by(|a, b| {
        (a.0.package_id(), &a.1).cmp(&(b.0.package_id(), &b.1))
    });

    let config = options.compile_opts.config;
    let cwd = options.compile_opts.config.cwd();

    let mut errors = vec![];

    let v : Vec<std::ffi::OsString> = test_args.iter().cloned().map::<std::ffi::OsString, _>(|val| val.into()).collect();

    //let x = &compilation.tests.map(run_single_coverage);

    for &(ref pkg, _, ref exe) in &compilation.tests {
        let to_display = match cargo::util::without_prefix(exe, &cwd) {
            Some(path) => path,
            None => &**exe
        };

        // DLYB trick on OSX is here v
        let mut cmd = try!(compilation.target_process("kcov", pkg));
        // TODO: Make all that more configurable
        //TODO: The unwraps shouldn't cause problems... right ?
        let target = "target/kcov-".to_string() + to_display.file_name().unwrap().to_str().unwrap();
        let mut args : Vec<&std::ffi::OsStr> = vec!["--verify".as_ref(), "--exclude-pattern=/.cargo".as_ref(), target.as_ref()];
        args.push(exe.as_os_str());
        let w : Vec<&std::ffi::OsStr> = v.iter().map(|v| v.as_os_str()).collect();
        args.extend(w);
        cmd.args(&args);
        try!(config.shell().concise(|shell| {
            shell.status("Running", to_display.display().to_string())
        }));
        try!(config.shell().verbose(|shell| {
            shell.status("Running", cmd.to_string())
        }));

        if let Err(e) = ExecEngine::exec(&ProcessEngine, cmd) {
            errors.push(e);
        }
    }

    // Let the user pass mergeargs
    let mut mergeargs : Vec<OsString> = vec!["--merge".to_string().into(), options.merge_dir.as_os_str().to_os_string()];
    mergeargs.extend(options.merge_args.iter().cloned());
    mergeargs.extend(compilation.tests.iter().map(|&(_, _, ref exe)|
        ("target/kcov-".to_string() + exe.file_name().unwrap().to_str().unwrap()).into()
    ));
    let mut cmd = try!(CommandPrototype::new(CommandType::Target("kcov".to_string().into()), options.compile_opts.config));
    cmd.args(&mergeargs);
    try!(config.shell().concise(|shell| {
        shell.status("Merging coverage", options.merge_dir.display().to_string())
    }));
    try!(config.shell().verbose(|shell| {
        shell.status("Merging coverage", cmd.to_string())
    }));
    try!(ExecEngine::exec(&ProcessEngine, cmd));
    if errors.is_empty() {
        Ok(None)
    } else {
        Ok(Some(CargoTestError::new(errors)))
    }
}