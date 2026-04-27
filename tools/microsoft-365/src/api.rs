use crate::graph::{self, OCTET_STREAM_MIME, SIMPLE_UPLOAD_LIMIT};
use serde::Serialize;

#[derive(Serialize)]
pub struct MeResult {
    pub id: String,
    pub display_name: String,
    pub mail: Option<String>,
    pub user_principal_name: Option<String>,
    pub job_title: Option<String>,
}

pub fn me() -> Result<MeResult, String> {
    graph::require_token()?;
    let (_status, value) = graph::request("GET", "/me", None)?;

    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Graph /me response missing id".to_string())?
        .to_string();
    let display_name = value
        .get("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mail = value
        .get("mail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let user_principal_name = value
        .get("userPrincipalName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let job_title = value
        .get("jobTitle")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(MeResult {
        id,
        display_name,
        mail,
        user_principal_name,
        job_title,
    })
}

#[derive(Serialize)]
pub struct SendMailResult {
    pub ok: bool,
    pub message: &'static str,
    pub unverified: bool,
}

pub fn send_mail(
    to: &[String],
    cc: &[String],
    bcc: &[String],
    subject: &str,
    body: &str,
    body_is_html: bool,
    save_to_sent_items: bool,
) -> Result<SendMailResult, String> {
    graph::require_token()?;
    if to.is_empty() {
        return Err("send_mail requires at least one recipient in `to`".to_string());
    }
    if subject.trim().is_empty() {
        return Err("send_mail requires a non-empty subject".to_string());
    }

    let to_recipients: Vec<_> = to.iter().map(|addr| recipient(addr)).collect();
    let cc_recipients: Vec<_> = cc.iter().map(|addr| recipient(addr)).collect();
    let bcc_recipients: Vec<_> = bcc.iter().map(|addr| recipient(addr)).collect();

    let payload = serde_json::json!({
        "message": {
            "subject": subject,
            "body": {
                "contentType": if body_is_html { "HTML" } else { "Text" },
                "content": body,
            },
            "toRecipients": to_recipients,
            "ccRecipients": cc_recipients,
            "bccRecipients": bcc_recipients,
        },
        "saveToSentItems": save_to_sent_items,
    });

    let body_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let (status, _value) = graph::request("POST", "/me/sendMail", Some(&body_str))?;

    // sendMail returns 202 Accepted with no body and no message-id. The send
    // is queued and fans out asynchronously, so we mark the result unverified.
    Ok(SendMailResult {
        ok: status == 202,
        message: "Send queued by Graph. Delivery happens asynchronously.",
        unverified: true,
    })
}

fn recipient(address: &str) -> serde_json::Value {
    serde_json::json!({ "emailAddress": { "address": address } })
}

#[derive(Serialize)]
pub struct MessageSummary {
    pub id: String,
    pub subject: String,
    pub from: Option<String>,
    pub received: Option<String>,
    pub is_read: bool,
    pub preview: String,
}

#[derive(Serialize)]
pub struct MessagesResult {
    pub count: usize,
    pub messages: Vec<MessageSummary>,
}

pub fn list_recent_messages(limit: u32, filter: Option<&str>) -> Result<MessagesResult, String> {
    graph::require_token()?;
    let capped = limit.clamp(1, 100);
    let mut endpoint = format!(
        "/me/messages?$top={}&$select=id,subject,from,receivedDateTime,isRead,bodyPreview&$orderby=receivedDateTime desc",
        capped
    );
    if let Some(f) = filter {
        if !f.trim().is_empty() {
            endpoint.push_str("&$filter=");
            endpoint.push_str(&graph::url_encode(f));
        }
    }

    let (_status, value) = graph::request("GET", &endpoint, None)?;

    let items = value
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Graph /me/messages response missing value array".to_string())?;

    let mut messages = Vec::with_capacity(items.len());
    for item in items {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let subject = item
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let from = item
            .get("from")
            .and_then(|f| f.get("emailAddress"))
            .and_then(|e| e.get("address"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let received = item
            .get("receivedDateTime")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let is_read = item
            .get("isRead")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let preview = item
            .get("bodyPreview")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        messages.push(MessageSummary {
            id,
            subject,
            from,
            received,
            is_read,
            preview,
        });
    }

    Ok(MessagesResult {
        count: messages.len(),
        messages,
    })
}

#[derive(Serialize)]
pub struct DriveItem {
    pub id: String,
    pub name: String,
    pub is_folder: bool,
    pub size: u64,
    pub last_modified: Option<String>,
    pub web_url: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Serialize)]
pub struct DriveListing {
    pub count: usize,
    pub items: Vec<DriveItem>,
}

pub fn list_drive(folder_id: Option<&str>) -> Result<DriveListing, String> {
    graph::require_token()?;
    let endpoint = match folder_id {
        Some(id) => format!("/me/drive/items/{}/children", id),
        None => "/me/drive/root/children".to_string(),
    };

    let (_, value) = graph::request("GET", &endpoint, None)?;
    let items_json = value
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Graph drive listing missing value array".to_string())?;

    let mut items = Vec::with_capacity(items_json.len());
    for item in items_json {
        items.push(DriveItem {
            id: item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            is_folder: item.get("folder").is_some(),
            size: item.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
            last_modified: item
                .get("lastModifiedDateTime")
                .and_then(|v| v.as_str())
                .map(String::from),
            web_url: item
                .get("webUrl")
                .and_then(|v| v.as_str())
                .map(String::from),
            mime_type: item
                .get("file")
                .and_then(|f| f.get("mimeType"))
                .and_then(|v| v.as_str())
                .map(String::from),
        });
    }

    Ok(DriveListing {
        count: items.len(),
        items,
    })
}

#[derive(Serialize)]
pub struct UploadedFile {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub web_url: Option<String>,
    pub download_url: Option<String>,
}

pub fn upload_file(
    parent_folder_id: Option<&str>,
    name: &str,
    content_base64: &str,
) -> Result<UploadedFile, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content_base64)
        .map_err(|e| format!("Invalid base64 content: {}", e))?;
    upload_to_drive(parent_folder_id, name, &bytes, OCTET_STREAM_MIME)
}

pub fn upload_to_drive(
    parent_folder_id: Option<&str>,
    name: &str,
    bytes: &[u8],
    content_type: &str,
) -> Result<UploadedFile, String> {
    graph::require_token()?;
    if name.trim().is_empty() {
        return Err("upload requires a non-empty name".to_string());
    }
    let value = upload_bytes(parent_folder_id, name, bytes, content_type)?;
    Ok(uploaded_file_from_value(value))
}

fn upload_bytes(
    parent_folder_id: Option<&str>,
    name: &str,
    bytes: &[u8],
    content_type: &str,
) -> Result<serde_json::Value, String> {
    if bytes.len() > SIMPLE_UPLOAD_LIMIT {
        return Err(format!(
            "{} bytes exceeds the {}-byte simple-upload limit; chunked upload session not yet supported",
            bytes.len(),
            SIMPLE_UPLOAD_LIMIT
        ));
    }
    let encoded_name = graph::url_encode(name);
    let endpoint = match parent_folder_id {
        Some(id) => format!("/me/drive/items/{}:/{}:/content", id, encoded_name),
        None => format!("/me/drive/root:/{}:/content", encoded_name),
    };
    graph::put_bytes(&endpoint, bytes, content_type)
}

fn uploaded_file_from_value(value: serde_json::Value) -> UploadedFile {
    UploadedFile {
        id: value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        size: value.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
        web_url: value
            .get("webUrl")
            .and_then(|v| v.as_str())
            .map(String::from),
        download_url: value
            .get("@microsoft.graph.downloadUrl")
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

#[derive(Serialize)]
pub struct ExcelRangeRead {
    pub address: String,
    pub row_count: u64,
    pub column_count: u64,
    pub values: serde_json::Value,
    pub text: serde_json::Value,
    pub formulas: serde_json::Value,
    pub number_format: serde_json::Value,
}

pub fn read_excel_range(
    workbook_id: &str,
    worksheet: &str,
    range: &str,
) -> Result<ExcelRangeRead, String> {
    graph::require_token()?;
    let endpoint = format!(
        "/me/drive/items/{}/workbook/worksheets('{}')/range(address='{}')",
        workbook_id,
        graph::url_encode(worksheet),
        graph::url_encode(range)
    );

    let (_, value) = graph::request("GET", &endpoint, None)?;

    Ok(ExcelRangeRead {
        address: value
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        row_count: value.get("rowCount").and_then(|v| v.as_u64()).unwrap_or(0),
        column_count: value
            .get("columnCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        values: value
            .get("values")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        text: value
            .get("text")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        formulas: value
            .get("formulas")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        number_format: value
            .get("numberFormat")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    })
}

#[derive(Serialize)]
pub struct ExcelRangeWrite {
    pub ok: bool,
    pub address: String,
    pub row_count: u64,
    pub column_count: u64,
}

pub fn write_excel_range(
    workbook_id: &str,
    worksheet: &str,
    range: &str,
    values: &[Vec<serde_json::Value>],
) -> Result<ExcelRangeWrite, String> {
    graph::require_token()?;
    if values.is_empty() {
        return Err("write_excel_range requires at least one row of values".to_string());
    }

    let endpoint = format!(
        "/me/drive/items/{}/workbook/worksheets('{}')/range(address='{}')",
        workbook_id,
        graph::url_encode(worksheet),
        graph::url_encode(range)
    );
    let payload = serde_json::json!({ "values": values });
    let body_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    let (_, value) = graph::request("PATCH", &endpoint, Some(&body_str))?;

    Ok(ExcelRangeWrite {
        ok: true,
        address: value
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        row_count: value.get("rowCount").and_then(|v| v.as_u64()).unwrap_or(0),
        column_count: value
            .get("columnCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

#[derive(Serialize)]
pub struct TeamSummary {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct TeamsListing {
    pub count: usize,
    pub teams: Vec<TeamSummary>,
}

pub fn list_teams() -> Result<TeamsListing, String> {
    graph::require_token()?;
    let (_, value) = graph::request("GET", "/me/joinedTeams", None)?;
    let items = value
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Graph joinedTeams response missing value array".to_string())?;

    let teams = items
        .iter()
        .map(|item| TeamSummary {
            id: item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            display_name: item
                .get("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: item
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
        })
        .collect::<Vec<_>>();

    Ok(TeamsListing {
        count: teams.len(),
        teams,
    })
}

#[derive(Serialize)]
pub struct ChannelSummary {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub membership_type: Option<String>,
}

#[derive(Serialize)]
pub struct ChannelsListing {
    pub count: usize,
    pub channels: Vec<ChannelSummary>,
}

pub fn list_channels(team_id: &str) -> Result<ChannelsListing, String> {
    graph::require_token()?;
    if team_id.trim().is_empty() {
        return Err("list_channels requires a team_id. Run list_teams first.".to_string());
    }
    let endpoint = format!("/teams/{}/channels", team_id);
    let (_, value) = graph::request("GET", &endpoint, None)?;
    let items = value
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Graph channels response missing value array".to_string())?;

    let channels = items
        .iter()
        .map(|item| ChannelSummary {
            id: item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            display_name: item
                .get("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: item
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            membership_type: item
                .get("membershipType")
                .and_then(|v| v.as_str())
                .map(String::from),
        })
        .collect::<Vec<_>>();

    Ok(ChannelsListing {
        count: channels.len(),
        channels,
    })
}

#[derive(Serialize)]
pub struct ChannelPostResult {
    pub id: String,
    pub created_date_time: Option<String>,
    pub web_url: Option<String>,
}

pub fn send_channel_message(
    team_id: &str,
    channel_id: &str,
    content: &str,
    content_is_html: bool,
) -> Result<ChannelPostResult, String> {
    graph::require_token()?;
    if team_id.trim().is_empty() || channel_id.trim().is_empty() {
        return Err("send_channel_message requires both team_id and channel_id".to_string());
    }
    if content.trim().is_empty() {
        return Err("send_channel_message requires non-empty content".to_string());
    }

    let endpoint = format!("/teams/{}/channels/{}/messages", team_id, channel_id);
    let payload = serde_json::json!({
        "body": {
            "contentType": if content_is_html { "html" } else { "text" },
            "content": content,
        }
    });
    let body_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    let (_, value) = graph::request("POST", &endpoint, Some(&body_str))?;

    Ok(ChannelPostResult {
        id: value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        created_date_time: value
            .get("createdDateTime")
            .and_then(|v| v.as_str())
            .map(String::from),
        web_url: value
            .get("webUrl")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

#[derive(Serialize)]
pub struct CalendarEventSummary {
    pub id: String,
    pub subject: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub time_zone: Option<String>,
    pub organizer: Option<String>,
    pub location: Option<String>,
    pub preview: Option<String>,
    pub web_link: Option<String>,
}

#[derive(Serialize)]
pub struct CalendarEventsListing {
    pub count: usize,
    pub events: Vec<CalendarEventSummary>,
}

pub fn list_calendar_events(
    start: Option<&str>,
    end: Option<&str>,
) -> Result<CalendarEventsListing, String> {
    graph::require_token()?;

    let endpoint = match (start, end) {
        (Some(s), Some(e)) if !s.is_empty() && !e.is_empty() => format!(
            "/me/calendarView?startDateTime={}&endDateTime={}&$top=50&$orderby=start/dateTime",
            graph::url_encode(s),
            graph::url_encode(e)
        ),
        _ => "/me/events?$top=50&$orderby=start/dateTime&$select=id,subject,start,end,organizer,location,bodyPreview,webLink".to_string(),
    };

    let (_, value) = graph::request("GET", &endpoint, None)?;
    let items = value
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Graph calendar response missing value array".to_string())?;

    let events = items
        .iter()
        .map(|item| {
            let start_dt = item
                .get("start")
                .and_then(|s| s.get("dateTime"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let end_dt = item
                .get("end")
                .and_then(|e| e.get("dateTime"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let time_zone = item
                .get("start")
                .and_then(|s| s.get("timeZone"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let organizer = item
                .get("organizer")
                .and_then(|o| o.get("emailAddress"))
                .and_then(|e| e.get("address"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let location = item
                .get("location")
                .and_then(|l| l.get("displayName"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from);
            let preview = item
                .get("bodyPreview")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from);
            let web_link = item
                .get("webLink")
                .and_then(|v| v.as_str())
                .map(String::from);

            CalendarEventSummary {
                id: item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                subject: item
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                start: start_dt,
                end: end_dt,
                time_zone,
                organizer,
                location,
                preview,
                web_link,
            }
        })
        .collect::<Vec<_>>();

    Ok(CalendarEventsListing {
        count: events.len(),
        events,
    })
}

#[derive(Serialize)]
pub struct CreatedEvent {
    pub id: String,
    pub subject: String,
    pub web_link: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}

pub fn create_calendar_event(
    subject: &str,
    start: &str,
    end: &str,
    attendees: &[String],
    location: Option<&str>,
    body: Option<&str>,
) -> Result<CreatedEvent, String> {
    graph::require_token()?;
    if subject.trim().is_empty() {
        return Err("create_calendar_event requires a subject".to_string());
    }
    if start.trim().is_empty() || end.trim().is_empty() {
        return Err(
            "create_calendar_event requires both start and end in ISO-8601 format".to_string(),
        );
    }

    let (start_dt, start_tz) = split_iso8601(start);
    let (end_dt, end_tz) = split_iso8601(end);

    let attendees_json: Vec<_> = attendees
        .iter()
        .map(|addr| {
            serde_json::json!({
                "emailAddress": { "address": addr },
                "type": "required"
            })
        })
        .collect();

    let mut payload = serde_json::json!({
        "subject": subject,
        "start": { "dateTime": start_dt, "timeZone": start_tz },
        "end": { "dateTime": end_dt, "timeZone": end_tz },
        "attendees": attendees_json,
    });

    if let Some(loc) = location {
        if !loc.trim().is_empty() {
            payload["location"] = serde_json::json!({ "displayName": loc });
        }
    }
    if let Some(b) = body {
        if !b.trim().is_empty() {
            payload["body"] = serde_json::json!({ "contentType": "text", "content": b });
        }
    }

    let body_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let (_, value) = graph::request("POST", "/me/events", Some(&body_str))?;

    Ok(CreatedEvent {
        id: value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        subject: value
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        web_link: value
            .get("webLink")
            .and_then(|v| v.as_str())
            .map(String::from),
        start: value
            .get("start")
            .and_then(|s| s.get("dateTime"))
            .and_then(|v| v.as_str())
            .map(String::from),
        end: value
            .get("end")
            .and_then(|e| e.get("dateTime"))
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

fn split_iso8601(ts: &str) -> (String, String) {
    if let Some(plus_idx) = ts.rfind('+') {
        if plus_idx > 10 {
            return (ts[..plus_idx].to_string(), ts[plus_idx..].to_string());
        }
    }
    if let Some(minus_idx) = ts[10..].rfind('-').map(|i| i + 10) {
        return (ts[..minus_idx].to_string(), ts[minus_idx..].to_string());
    }
    if ts.ends_with('Z') {
        return (ts.trim_end_matches('Z').to_string(), "UTC".to_string());
    }
    (ts.to_string(), "UTC".to_string())
}
