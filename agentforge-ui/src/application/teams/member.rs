use rusqlite::{params, Connection, Result};

pub struct Member {
    pub id: String,
    pub team_id: String,
    pub instance_id: Option<String>,
    pub agent_id: String,
    pub role_id: Option<String>,
    pub joined_at: String,
}

pub struct MemberManager<'a> {
    conn: &'a Connection,
}

impl<'a> MemberManager<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn add_member(&self, member: &Member) -> Result<()> {
        self.conn.execute(
            "INSERT INTO members (id, team_id, instance_id, agent_id, role_id, joined_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                member.id,
                member.team_id,
                member.instance_id,
                member.agent_id,
                member.role_id,
                member.joined_at
            ],
        )?;
        Ok(())
    }

    pub fn remove_member(&self, member_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM members WHERE id = ?1", params![member_id])?;
        Ok(())
    }

    pub fn reassign_role(&self, member_id: &str, new_role_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE members SET role_id = ?1 WHERE id = ?2",
            params![new_role_id, member_id],
        )?;
        Ok(())
    }

    pub fn list_members(&self, team_id: &str) -> Result<Vec<Member>> {
        let mut stmt = self.conn.prepare("SELECT id, team_id, instance_id, agent_id, role_id, joined_at FROM members WHERE team_id = ?1")?;
        let iter = stmt.query_map(params![team_id], |row| {
            Ok(Member {
                id: row.get(0)?,
                team_id: row.get(1)?,
                instance_id: row.get(2)?,
                agent_id: row.get(3)?,
                role_id: row.get(4)?,
                joined_at: row.get(5)?,
            })
        })?;

        let mut members = Vec::new();
        for m in iter {
            members.push(m?);
        }
        Ok(members)
    }
}
