use xtask::codegen;

#[test]
fn generated_github_workflows_are_fresh() {
    codegen::github_workflows::generate(codegen::Mode::Verify).unwrap();
}
