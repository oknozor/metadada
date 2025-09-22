use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, serde::Deserialize)]
pub struct PendingData {
    seqid: i32,
    fulltable: String,
    op: char,
    xid: String,
    olddata: String,
    newdata: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PendindingKey {
    fulltable: String,
    keys: String,
}

impl PendindingKey {
    pub fn into_entry(self) -> (String, Vec<String>) {
        (
            self.fulltable,
            self.keys
                .split(',')
                .map(|s| s.trim_matches(['{', '}']))
                .map(String::from)
                .collect(),
        )
    }
}

impl PendingData {
    pub fn to_sql_inline(
        &self,
        pending_keys: &BTreeMap<String, Vec<String>>,
    ) -> anyhow::Result<Option<String>> {
        let (schema, table) = self.split_table_schema();
        match self.op {
            'i' => {
                let obj = self
                    .newdata()
                    .ok_or_else(|| anyhow::anyhow!("missing data"))?;
                let obj = obj.as_object().unwrap();
                let col_names: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
                let col_values: Vec<String> = obj.values().map(sql_literal).collect();
                Ok(Some(format!(
                    r#"INSERT INTO "{schema}"."{table}" ({}) VALUES ({});"#,
                    col_names.join(", "),
                    col_values.join(", ")
                )))
            }
            'u' => {
                let ids = pending_keys
                    .get(&self.fulltable)
                    .expect("missing pending key");

                let Some(set_clause) = self.get_set_clause()? else {
                    return Ok(None);
                };

                let where_clause = self.get_where_clause(ids)?;

                Ok(Some(format!(
                    r#"UPDATE "{schema}"."{table}" SET {set_clause} WHERE {where_clause};"#,
                )))
            }
            'd' => {
                let ids = pending_keys
                    .get(&self.fulltable)
                    .expect("missing pending key");

                let where_clause = self.get_where_clause(ids)?;

                Ok(Some(format!(
                    r#"DELETE FROM "{schema}"."{table}" WHERE {where_clause};"#,
                )))
            }
            _ => anyhow::bail!("Invalid operation: {}", self.op),
        }
    }

    pub fn split_table_schema(&self) -> (&str, &str) {
        let parts: Vec<&str> = self.fulltable.split('.').collect();
        (parts[0], parts[1])
    }

    fn newdata(&self) -> Option<Value> {
        self.newdata
            .as_ref()
            .map(|s| s.replace("\\\\", "\\"))
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    fn olddata(&self) -> Option<Value> {
        let olddata = self.olddata.replace("\\\\", "\\");
        serde_json::from_str(&olddata).ok()
    }

    fn get_where_clause(&self, ids: &[String]) -> anyhow::Result<String> {
        let old_obj = self
            .olddata()
            .ok_or_else(|| anyhow::anyhow!("missing data"))?;

        let old_obj = old_obj.as_object().unwrap();

        let where_clause: Vec<String> = old_obj
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, v)| format!("{k} = {}", sql_literal(v)))
            .collect();

        Ok(where_clause.join(" AND "))
    }

    fn get_set_clause(&self) -> anyhow::Result<Option<String>> {
        let new_obj = self
            .newdata()
            .ok_or_else(|| anyhow::anyhow!("missing newdata"))?;
        let new_obj = new_obj.as_object().unwrap();

        let old_obj = self
            .olddata()
            .ok_or_else(|| anyhow::anyhow!("missing data"))?;
        let old_obj = old_obj.as_object().unwrap();

        let mut changes = Vec::new();

        for (k, new_val) in new_obj {
            if let Some(old_val) = old_obj.get(k) {
                if old_val != new_val {
                    changes.push(format!(r#"{k} = {}"#, sql_literal(new_val)));
                }
            } else {
                changes.push(format!(r#"{k} = {}"#, sql_literal(new_val)));
            }
        }

        if changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(changes.join(", ")))
    }
}

fn sql_literal(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Array(arr) => {
            let elems: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.replace('\'', "''"),
                    _ => panic!("unsupported array element: {:?}", v),
                })
                .collect();
            format!("'{{{}}}'::integer[]", elems.join(","))
        }
        serde_json::Value::Object(_) => {
            format!("'{}'", val.to_string().replace('\'', "''"))
        }
    }
}
pub fn sort_pending_data(data: Vec<PendingData>) -> Vec<PendingData> {
    let mut data = data;
    data.sort_by(|a, b| {
        let xid_cmp = a.xid.cmp(&b.xid);
        if xid_cmp != std::cmp::Ordering::Equal {
            xid_cmp
        } else {
            a.seqid.cmp(&b.seqid)
        }
    });
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_release_group_meta_no_changes() -> anyhow::Result<()> {
        let olddata = serde_json::json!({
            "id": 3183888,
            "release_count": 3,
            "first_release_date_year": 2022,
            "first_release_date_month": 9,
            "first_release_date_day": 30,
            "rating": null,
            "rating_count": null
        });

        let newdata = serde_json::json!({
            "id": 3183888,
            "release_count": 3,
            "first_release_date_year": 2022,
            "first_release_date_month": 9,
            "first_release_date_day": 30,
            "rating": null,
            "rating_count": null
        });

        let pd = PendingData {
            seqid: 299795840,
            fulltable: "musicbrainz.release_group_meta".to_string(),
            op: 'u',
            xid: "2266759644".to_string(),
            olddata: olddata.to_string(),
            newdata: Some(newdata.to_string()),
        };

        let mut pk = BTreeMap::new();
        pk.insert(
            "musicbrainz.release_group_meta".to_string(),
            vec!["id".to_string()],
        );

        let query = pd.to_sql_inline(&pk)?;

        assert!(query.is_none());
        Ok(())
    }
}
