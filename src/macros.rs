/// use database_type to create a "DataValue"
///
/// - @Number -> 1
/// - @String -> "hello world".to_string()
/// - @Dict -> HashMap::new()
/// - @Boolean -> false
#[macro_export]
macro_rules! database_type {
    (
        @$key:ident : $value:expr
    ) => {
        DataValue::$key($value)
    };
}

/// use for get type: "DataValue::Dict()"
/// 
/// example:
/// ```
/// dict!(
/// "username" => "liuzhuoer",
/// "password" => "123456789",
/// "email" => "mrxzx@qq.com"
/// )
/// ```
#[macro_export]
macro_rules! dict {
    (
        $($key: expr => $value: expr),*
    ) => {
        {
            let mut map = HashMap::new();
            $( map.insert($key, $value); )*
            map
        }
    };
}