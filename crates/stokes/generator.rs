use erg_common::config::ErgConfig;
use erg_common::pathutil::NormalizedPathBuf;
use erg_compiler::module::SharedCompilerResource;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Module {
    name: String,
    description: Option<String>,
    subroutines: Vec<Subroutine>,
    classes: Vec<Class>,
    attrs: Vec<Variable>,
}

impl Module {
    fn sort(&mut self) {
        self.subroutines.sort_by(|a, b| a.name.cmp(&b.name));
        self.classes.sort_by(|a, b| a.name.cmp(&b.name));
        self.attrs.sort_by(|a, b| a.name.cmp(&b.name));
        for class in self.classes.iter_mut() {
            class.sort();
        }
    }
}

#[derive(Debug, Serialize)]
struct Subroutine {
    name: String,
    description: Option<String>,
    typ: String,
    example: Option<String>,
    output: Option<String>,
}
#[derive(Debug, Serialize)]
struct Class {
    name: String,
    description: Option<String>,
    instance_attrs: Vec<Variable>,
    class_attrs: Vec<Variable>,
    methods: Vec<Subroutine>,
}

impl Class {
    fn sort(&mut self) {
        self.instance_attrs.sort_by(|a, b| a.name.cmp(&b.name));
        self.class_attrs.sort_by(|a, b| a.name.cmp(&b.name));
        self.methods.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

#[derive(Debug, Serialize)]
struct Variable {
    name: String,
    typ: String,
    description: Option<String>,
}

#[derive(Debug)]
pub struct HTMLGenerator {
    pub config: ErgConfig,
}

impl HTMLGenerator {
    pub fn new(config: ErgConfig) -> Self {
        HTMLGenerator { config }
    }

    pub fn generate_builtin(&mut self) {
        let mut module = Module {
            name: "builtins".to_string(),
            description: Some("Built-in functions and classes".to_string()),
            subroutines: vec![],
            classes: vec![],
            attrs: vec![],
        };
        let shared = SharedCompilerResource::new(self.config.clone());
        let ctx = shared
            .mod_cache
            .remove(&NormalizedPathBuf::from("<builtins>"))
            .unwrap()
            .module
            .context;
        for (name, ctx) in ctx.types() {
            // let description = ctx.def_loc.code();
            let mut class = Class {
                name: name.to_string(),
                description: None,
                instance_attrs: vec![],
                methods: vec![],
                class_attrs: vec![],
            };
            for (name, vi) in ctx.instance_attrs() {
                let description = vi.def_loc.code();
                let var = Variable {
                    name: name.to_string(),
                    description,
                    typ: vi.t.to_string(),
                };
                class.instance_attrs.push(var);
            }
            for (name, vi) in ctx.class_attrs() {
                let description = vi.def_loc.code();
                let var = Variable {
                    name: name.to_string(),
                    description,
                    typ: vi.t.to_string(),
                };
                class.class_attrs.push(var);
            }
            for (name, vi) in ctx.all_methods() {
                let description = vi.def_loc.code();
                let sub = Subroutine {
                    name: name.to_string(),
                    description,
                    typ: vi.t.to_string(),
                    example: None,
                    output: None,
                };
                class.methods.push(sub);
            }
            module.classes.push(class);
        }
        for (name, vi) in ctx.dir() {
            if ctx.has_type(name.inspect()) {
                continue;
            }
            if vi.t.is_subr() {
                let description = vi.def_loc.code();
                let sub = Subroutine {
                    name: name.to_string(),
                    description,
                    typ: vi.t.to_string(),
                    example: None,
                    output: None,
                };
                module.subroutines.push(sub);
            } else {
                // let description = vi.def_loc.code();
                let typ = vi.t.to_string();
                let var = Variable {
                    name: name.to_string(),
                    description: None,
                    typ,
                };
                module.attrs.push(var);
            }
        }
        module.sort();
        let mut env = minijinja::Environment::new();
        env.add_template("module.html", include_str!("template/module.html"))
            .unwrap();
        let template = env.get_template("module.html").unwrap();
        let rendered = template
            .render(minijinja::context! { module => module })
            .unwrap();
        if !std::path::Path::new("build/docs").exists() {
            std::fs::create_dir_all("build/docs").unwrap();
        }
        std::fs::copy("template/style.css", "build/docs/style.css").unwrap();
        std::fs::copy("template/darcula.css", "build/docs/darcula.css").unwrap();
        std::fs::copy("template/prism.js", "build/docs/prism.js").unwrap();
        std::fs::copy("template/search.js", "build/docs/search.js").unwrap();
        std::fs::write("build/docs/builtins.html", rendered).unwrap();
    }
}
