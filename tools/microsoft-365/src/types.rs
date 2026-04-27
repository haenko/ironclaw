use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum MicrosoftAction {
    Me,
    SendMail {
        to: Vec<String>,
        #[serde(default)]
        cc: Vec<String>,
        #[serde(default)]
        bcc: Vec<String>,
        subject: String,
        body: String,
        #[serde(default)]
        body_is_html: bool,
        #[serde(default = "default_true")]
        save_to_sent_items: bool,
    },
    ListRecentMessages {
        #[serde(default = "default_mail_limit")]
        limit: u32,
        #[serde(default)]
        filter: Option<String>,
    },
    SendChannelMessage {
        team_id: String,
        channel_id: String,
        content: String,
        #[serde(default)]
        content_is_html: bool,
    },
    ListTeams,
    ListChannels {
        team_id: String,
    },
    ReadExcelRange {
        workbook_id: String,
        worksheet: String,
        range: String,
    },
    WriteExcelRange {
        workbook_id: String,
        worksheet: String,
        range: String,
        values: Vec<Vec<serde_json::Value>>,
    },
    ListDrive {
        #[serde(default)]
        folder_id: Option<String>,
    },
    UploadFile {
        #[serde(default)]
        parent_folder_id: Option<String>,
        name: String,
        content_base64: String,
    },
    ListCalendarEvents {
        #[serde(default)]
        start: Option<String>,
        #[serde(default)]
        end: Option<String>,
    },
    CreateCalendarEvent {
        subject: String,
        start: String,
        end: String,
        #[serde(default)]
        attendees: Vec<String>,
        #[serde(default)]
        location: Option<String>,
        #[serde(default)]
        body: Option<String>,
    },
    CreateWordDocument {
        #[serde(default)]
        parent_folder_id: Option<String>,
        name: String,
        title: String,
        #[serde(default)]
        subtitle: Option<String>,
        #[serde(default)]
        sections: Vec<WordSection>,
    },
    CreatePowerpoint {
        #[serde(default)]
        parent_folder_id: Option<String>,
        name: String,
        slides: Vec<PowerpointSlide>,
    },
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WordSection {
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub paragraphs: Vec<String>,
    #[serde(default)]
    pub table: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PowerpointSlide {
    pub title: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

fn default_mail_limit() -> u32 {
    25
}

fn default_true() -> bool {
    true
}
