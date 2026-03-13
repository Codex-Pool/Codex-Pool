#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenPriceMicrousd {
    pub input_price_microusd: i64,
    pub cached_input_price_microusd: i64,
    pub output_price_microusd: i64,
}

const PRICE_PER_MILLION_TOKENS_SCALE: i128 = 1_000_000;

pub fn charge_tokens_by_per_million_price(tokens: i64, price_per_million_microusd: i64) -> i64 {
    let normalized_tokens = tokens.max(0) as i128;
    let normalized_price = price_per_million_microusd.max(0) as i128;
    if normalized_tokens == 0 || normalized_price == 0 {
        return 0;
    }

    // Round to nearest microusd to avoid systematic drift on very small requests.
    let numerator = normalized_tokens
        .saturating_mul(normalized_price)
        .saturating_add(PRICE_PER_MILLION_TOKENS_SCALE / 2);
    let charged = numerator / PRICE_PER_MILLION_TOKENS_SCALE;
    charged.clamp(0, i64::MAX as i128) as i64
}

pub fn calculate_estimated_cost_microusd(
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    pricing: TokenPriceMicrousd,
) -> i64 {
    let normalized_input_tokens = input_tokens.max(0);
    let normalized_cached_input_tokens = cached_input_tokens.max(0).min(normalized_input_tokens);
    let billable_input_tokens = normalized_input_tokens.saturating_sub(normalized_cached_input_tokens);
    let normalized_output_tokens = output_tokens.max(0);

    let input_charge = charge_tokens_by_per_million_price(
        billable_input_tokens,
        pricing.input_price_microusd,
    );
    let cached_input_charge = charge_tokens_by_per_million_price(
        normalized_cached_input_tokens,
        pricing.cached_input_price_microusd,
    );
    let output_charge = charge_tokens_by_per_million_price(
        normalized_output_tokens,
        pricing.output_price_microusd,
    );
    input_charge
        .saturating_add(cached_input_charge)
        .saturating_add(output_charge)
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_estimated_cost_microusd, charge_tokens_by_per_million_price, TokenPriceMicrousd,
    };

    #[test]
    fn charge_tokens_by_per_million_price_rounds_to_nearest_microusd() {
        assert_eq!(charge_tokens_by_per_million_price(1, 1_250_000), 1);
        assert_eq!(charge_tokens_by_per_million_price(2, 1_250_000), 3);
    }

    #[test]
    fn calculate_estimated_cost_microusd_applies_cached_input_discount() {
        let pricing = TokenPriceMicrousd {
            input_price_microusd: 1_250_000,
            cached_input_price_microusd: 125_000,
            output_price_microusd: 10_000_000,
        };

        assert_eq!(
            calculate_estimated_cost_microusd(800, 100, 400, pricing),
            4_888,
        );
    }
}
