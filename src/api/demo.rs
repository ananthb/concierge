//! Static landing-page demo fixtures and `POST /api/demo/chat`.

use serde::Serialize;
use worker::*;

#[derive(Serialize, Clone, Copy, Debug)]
pub struct DemoFixture {
    pub id: &'static str,
    pub business_name: &'static str,
    pub business_type: &'static str,
    pub city: &'static str,
    pub hours: &'static str,
    pub opener: &'static str,
    pub summary: &'static str,
}

pub const DEMO_MAX_USER_TURNS: u32 = 6;
pub const DEMO_IDLE_TIMEOUT_SECS: u32 = 300;

pub const FIXTURES: &[DemoFixture] = &[
    DemoFixture {
        id: "bakery",
        business_name: "Nutmeg & Crumb Bakery",
        business_type: "Neighborhood sourdough & pastry bakery",
        city: "Bengaluru",
        hours: "Tue–Sun, 8:00 AM – 8:00 PM",
        opener: "Hi! Welcome to Nutmeg & Crumb Bakery. How can we help today?",
        summary: "Artisan sourdough loaves, croissants, and custom celebration cakes baked fresh daily in Indiranagar, Bengaluru.",
    },
    DemoFixture {
        id: "clinic",
        business_name: "ClearSmile Dental Clinic",
        business_type: "Family & cosmetic dental clinic",
        city: "Pune",
        hours: "Mon–Sat, 9:30 AM – 7:30 PM",
        opener: "Hello from ClearSmile Dental Clinic! How can we assist you?",
        summary: "Routine checkups, cleanings, aligners, and gentle family dentistry in Kothrud, Pune. Appointments and treatment quotes are confirmed by our front desk.",
    },
    DemoFixture {
        id: "plumber",
        business_name: "QuickFix Home Plumbing",
        business_type: "Residential plumbing & leak repair",
        city: "Hyderabad",
        hours: "Daily, 7:00 AM – 9:00 PM",
        opener: "Hi there! QuickFix Home Plumbing here. What issue can we help with?",
        summary: "Same-day tap, pipe, water-heater, and drain repairs across Jubilee Hills and Madhapur, Hyderabad.",
    },
];

pub fn find_fixture(id: &str) -> Option<&'static DemoFixture> {
    let key = id.trim();
    FIXTURES.iter().find(|f| f.id == key)
}

pub async fn chat(req: Request, env: Env) -> Result<Response> {
    crate::handlers::handle_demo_chat(req, env).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_demo_fixtures_lookup_works() {
        assert_eq!(FIXTURES.len(), 3);
        assert_eq!(
            find_fixture("bakery").unwrap().business_name,
            "Nutmeg & Crumb Bakery"
        );
        assert_eq!(find_fixture("clinic").unwrap().city, "Pune");
        assert_eq!(find_fixture("plumber").unwrap().city, "Hyderabad");
        assert!(find_fixture("unknown").is_none());
    }
}
