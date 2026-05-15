mod component;
mod field;
mod proxy_method;
mod registration;
mod utils;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

/// # Component Derive Macro
///
/// 提供 `#[derive(Component)]` 自動生成 IoC 元件註冊與建構邏輯。
///
/// ## 功能
/// 1. 自動實作 `ioc_lite::Component` trait
/// 2. 自動生成 `create(scope)` 建構函式
/// 3. 支援欄位注入：
///    - `#[component]`：從 IoC 取得依賴（必須為 Proxy<T>）
///    - `#[value = "..."]`：常數注入
///    - `#[script(async fn(meta: Arc<Json>) -> T)`：腳本注入
///    - 無標註：使用 Default
/// 4. 自動註冊至 `inventory`
///
/// ## 限制
/// - 僅支援 struct
/// - 不支援 generic struct
/// - 不支援 tuple / unnamed struct
/// - component 欄位必須為 `Proxy<T>`
///
/// ## 使用範例
/// ```rust
/// use ioc_lite_macro::proxy_method;
///
///  async fn init_depend() -> Vec<Depend> { ... }
///
/// #[derive(Component)]
/// #[lifecycle = "ScopeNameRegex"] // Singleton | Singleton(Lazy) | Prototype // Singleton is default
/// struct Foo {
///     #[component]
///     service: Proxy<Service>,
///
///     #[value = "hello"]
///     name: String,
///
///     #[script(init_depend, cache)] // 啟用 cache 時回傳值必須可安全重用: T: Clone (可複製）| Arc<T>（共享不可變）
///     depend: Vec<Depend>,
///
///     #[script(async |_| vec![1, 2, 3])]
///     arr: Vec<i32>,
///
///     cache: Cache, // Default::default()
/// }
///
/// #[proxy_method] // 自動實作 Bean<Foo> 將 pub [async] fn(&[mut] self) -> T 包裝成 pub async fn(&self) -> T
/// impl Foo { ... }
/// ```
#[proc_macro_derive(Component, attributes(component, value, script, lifecycle))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    component::ComponentIR::from(input)
        .map(|ir| ir.token().into())
        .unwrap_or_else(|error| error.to_compile_error().into())
}

#[proc_macro_attribute]
pub fn proxy_method(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);
    proxy_method::expand_method(item).into()
}
