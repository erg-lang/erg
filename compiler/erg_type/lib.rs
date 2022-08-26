/// cloneのコストがあるためなるべく.ref_tを使うようにすること
/// いくつかの構造体は直接Typeを保持していないので、その場合は.tを使う
#[allow(unused_variables)]
pub trait HasType {
    fn ref_t(&self) -> &Type;
    // 関数呼び出しの場合、.ref_t()は戻り値を返し、signature_t()は関数全体の型を返す
    fn signature_t(&self) -> Option<&Type>;
    // 最後にHIR全体の型変数を消すために使う
    fn ref_mut_t(&mut self) -> &mut Type;
    fn signature_mut_t(&mut self) -> Option<&mut Type>;
    #[inline]
    fn t(&self) -> Type {
        self.ref_t().clone()
    }
    #[inline]
    fn inner_ts(&self) -> Vec<Type> {
        self.ref_t().inner_ts()
    }
    #[inline]
    fn lhs_t(&self) -> &Type {
        &self.ref_t().non_default_params().unwrap()[0].ty
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        &self.ref_t().non_default_params().unwrap()[1].ty
    }
}
