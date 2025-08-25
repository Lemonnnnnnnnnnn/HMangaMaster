use regex::Regex;
use std::collections::HashMap;

/// 纯Rust版本的GG结构体
#[derive(Debug, Clone)]
pub struct GGRust {
    pub b: String,
    pub m_map: HashMap<u32, u32>,
}

impl GGRust {
    /// 从gg.js字符串创建GGRust实例
    pub fn from_gg_js(gg_js: &str) -> anyhow::Result<Self> {
        let b = Self::extract_b_value(gg_js)?;
        let m_map = Self::extract_m_function(gg_js)?;

        Ok(GGRust { b, m_map })
    }

    /// 提取b属性的值
    fn extract_b_value(gg_js: &str) -> anyhow::Result<String> {
        // 匹配 b: 'value' 或 b: "value"
        let re = Regex::new(r#"b\s*:\s*['"]([^'"]+)['"]"#)?;
        if let Some(captures) = re.captures(gg_js) {
            if let Some(b_value) = captures.get(1) {
                return Ok(b_value.as_str().to_string());
            }
        }
        Err(anyhow::anyhow!("无法找到b属性值"))
    }

    /// 提取m函数的switch语句映射
    fn extract_m_function(gg_js: &str) -> anyhow::Result<HashMap<u32, u32>> {
        let mut m_map = HashMap::new();

        // 查找所有的 o = 数字; break; 语句
        let return_re = Regex::new(r"o\s*=\s*(\d+)")?;

        for captures in return_re.captures_iter(gg_js) {
            let return_value: u32 = captures
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("无法提取返回值"))?
                .as_str()
                .parse()?;

            // 获取当前匹配的位置
            let return_start = captures.get(0).unwrap().start();

            // 向后查找所有相关的 case 语句
            let before_text = &gg_js[0..return_start];

            // 查找从最后一个 "switch" 到当前 "o =" 之间的所有 case 语句
            if let Some(switch_pos) = before_text.rfind("switch") {
                let switch_to_return = &before_text[switch_pos..];

                // 匹配所有的 case 数字:
                let case_re = Regex::new(r"case\s+(\d+):")?;
                for case_capture in case_re.captures_iter(switch_to_return) {
                    if let Some(case_num_str) = case_capture.get(1) {
                        let case_value: u32 = case_num_str.as_str().parse()?;
                        m_map.insert(case_value, return_value);
                    }
                }
            }
        }

        Ok(m_map)
    }

    /// 实现gg.m()函数
    pub fn m(&self, value: u32) -> u32 {
        self.m_map.get(&value).copied().unwrap_or(0)
    }

    /// 实现gg.s()函数 - 从hash中提取数字
    /// 对应JavaScript: s: function(h) { var m = /(..)(.)$/.exec(h); return parseInt(m[2]+m[1], 16).toString(10); }
    pub fn s(&self, hash: &str) -> u32 {
        if hash.len() < 3 {
            return 0;
        }

        // 使用正则表达式匹配最后3个字符：/(..)(.)$/
        // 对于"7FF": m[1] = "7F" (倒数第2-3个字符), m[2] = "F" (最后1个字符)
        let last_three = &hash[hash.len().saturating_sub(3)..];
        if last_three.len() == 3 {
            // JavaScript正则表达式 /(..)(.)$/:
            // m[1] = 最后两个字符中的前两个字符 (即倒数第2-3个字符)
            let m1 = &last_three[0..2]; // "7FF" -> "7F"
                                        // m[2] = 最后一个字符
            let m2 = &last_three[2..]; // "7FF" -> "F"

            // 返回 parseInt(m[2] + m[1], 16) = parseInt("F" + "7F", 16)
            let combined = format!("{}{}", m2, m1);

            // 只有当组合的字符串是有效的16进制时才转换
            if combined.chars().all(|c| c.is_ascii_hexdigit()) {
                u32::from_str_radix(&combined, 16).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        }
    }
}

/// 新的gg.js解析函数（替代原有的parse_gg_constants）
pub fn parse_gg_constants_rust(gg_js: &str) -> anyhow::Result<GGRust> {
    let gg = GGRust::from_gg_js(gg_js)?;

    Ok(gg)
}

// 包含实际的gg.js文件内容作为测试数据
#[cfg(test)]
mod tests {
    use super::*;
    const GG_JS_CONTENT: &str = include_str!("./gg.js");

    #[test]
    fn test_gg_js_parsing() {
        // 测试gg.js解析功能
        let result = parse_gg_constants_rust(GG_JS_CONTENT);
        assert!(result.is_ok(), "gg.js解析应该成功");

        let gg = result.unwrap();

        // 验证b值
        assert_eq!(gg.b, "1756044001/", "b值应该匹配");

        // 验证m函数映射数量
        assert!(gg.m_map.len() > 0, "应该有m函数映射");
    }

    #[test]
    fn test_gg_m_function() {
        let gg = parse_gg_constants_rust(GG_JS_CONTENT).unwrap();

        // 测试一些已知的case值
        let test_cases = vec![
            (119, 1),
            (834, 1),
            (171, 1),
            (2801, 1),
            (3851, 1),
            (3532, 1),
            (1779, 1),
            (2332, 1),
            (1237, 0),
            (4412, 0),
            (5512, 0),
            (5452, 0),
        ];

        for (input, expected) in test_cases {
            let result = gg.m(input);
            assert_eq!(result, expected, "gg.m({}) 应该返回 {}", input, expected);
        }
    }

    #[test]
    fn test_gg_s_function() {
        let gg = parse_gg_constants_rust(GG_JS_CONTENT).unwrap();
        // JS: "4F2" -> "24F"(hex) -> 591(dec)
        assert_eq!(gg.s("4F2"), 591);

        // JS: "12AB3" -> "3AB"(hex) -> 939(dec)
        assert_eq!(gg.s("12AB3"), 939);

        // JS: "7FF" -> "F7F"(hex) -> 3967(dec)
        assert_eq!(gg.s("7FF"), 3967);

        // 长度不足3
        assert_eq!(gg.s("AB"), 0);

        // 非16进制字符
        assert_eq!(gg.s("12G"), 0);

        assert!(gg.s("123") == 0x312);

        assert!(gg.s("1234") == 0x423);
    }

    #[test]
    fn test_error_handling() {
        // 测试无效的gg.js内容
        let invalid_gg_js = "invalid javascript content";

        let result = parse_gg_constants_rust(invalid_gg_js);
        assert!(result.is_err(), "无效的gg.js应该导致错误");
    }
}
