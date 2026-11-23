//! Barra dei menu nativa di macOS (application-menu, "Menu su macOS"; ui-usability-audit D6).
//! L'interfaccia invia le voci del registro dei comandi (etichette già tradotte, scorciatoia,
//! disponibilità); il backend costruisce il menu dell'app più File, Modifica, Vista, Vai,
//! Strumenti e Aiuto. Un clic arriva all'interfaccia come evento `menu://command` con l'id del
//! comando. Su Windows e Linux il menu è disegnato dall'interfaccia e questo comando non fa nulla.

use pc_core::{AppError, AppResult};
use serde::Deserialize;
use tauri::menu::{
    CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, Submenu,
    SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Wry};

/// Evento con l'id del comando scelto dal menu nativo.
pub const MENU_COMMAND: &str = "menu://command";

/// Una voce di menu dal registro dei comandi.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MenuItemSpec {
    pub id: String,
    pub label: String,
    /// "file", "edit", "view", "go", "tools" o "help".
    pub menu: String,
    pub section: u32,
    pub order: u32,
    /// Voce raggruppata sotto un sottomenu con questo nome (es. "Tema").
    pub submenu: Option<String>,
    /// Scorciatoia nel formato di Tauri (es. "CmdOrCtrl+Shift+I").
    pub accelerator: Option<String>,
    pub enabled: bool,
    pub checked: Option<bool>,
}

/// Nomi tradotti dei menu e delle voci del menu dell'app.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MenuLabels {
    pub app_name: String,
    pub about: String,
    pub settings: String,
    pub quit: String,
    /// Nomi dei menu nell'ordine: File, Modifica, Vista, Vai, Strumenti, Aiuto.
    pub menus: Vec<(String, String)>,
}

/// Una riga di un menu: voce, separatore o sottomenu.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Item(MenuItemSpec),
    Separator,
    Submenu(String, Vec<MenuItemSpec>),
}

impl PartialEq for MenuItemSpec {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Righe di ogni menu, nell'ordine dei menu: sezioni separate, voci ordinate, sottomenu
/// raggruppati nel punto della loro prima voce. Pura: si verifica senza un menu vero.
pub fn layout(labels: &MenuLabels, items: &[MenuItemSpec]) -> Vec<(String, Vec<Row>)> {
    labels
        .menus
        .iter()
        .map(|(id, title)| {
            let mut mine: Vec<&MenuItemSpec> = items.iter().filter(|i| &i.menu == id).collect();
            mine.sort_by_key(|i| (i.section, i.order));
            let mut rows: Vec<Row> = Vec::new();
            let mut section = None;
            for item in &mine {
                if section.is_some_and(|s| s != item.section) {
                    rows.push(Row::Separator);
                }
                section = Some(item.section);
                match &item.submenu {
                    None => rows.push(Row::Item((*item).clone())),
                    Some(name) => {
                        let existing = rows.iter_mut().find_map(|r| match r {
                            Row::Submenu(n, list) if n == name => Some(list),
                            _ => None,
                        });
                        match existing {
                            Some(list) => list.push((*item).clone()),
                            None => rows.push(Row::Submenu(name.clone(), vec![(*item).clone()])),
                        }
                    }
                }
            }
            (title.clone(), rows)
        })
        .collect()
}

fn menu_error(e: tauri::Error) -> AppError {
    AppError::internal(format!("menu: {e}"))
}

fn add_items<'a>(
    app: &'a AppHandle,
    mut builder: SubmenuBuilder<'a, Wry, AppHandle>,
    rows: &[Row],
) -> AppResult<SubmenuBuilder<'a, Wry, AppHandle>> {
    for row in rows {
        builder = match row {
            Row::Separator => builder.separator(),
            Row::Item(item) => add_item(app, builder, item)?,
            Row::Submenu(name, list) => {
                let mut sub = SubmenuBuilder::new(app, name);
                for item in list {
                    sub = add_item(app, sub, item)?;
                }
                builder.item(&sub.build().map_err(menu_error)?)
            }
        };
    }
    Ok(builder)
}

fn add_item<'a>(
    app: &AppHandle,
    builder: SubmenuBuilder<'a, Wry, AppHandle>,
    item: &MenuItemSpec,
) -> AppResult<SubmenuBuilder<'a, Wry, AppHandle>> {
    Ok(match item.checked {
        Some(checked) => {
            let mut b = CheckMenuItemBuilder::with_id(&item.id, &item.label)
                .checked(checked)
                .enabled(item.enabled);
            if let Some(acc) = &item.accelerator {
                b = b.accelerator(acc);
            }
            builder.item(&b.build(app).map_err(menu_error)?)
        }
        None => {
            let mut b = MenuItemBuilder::with_id(&item.id, &item.label).enabled(item.enabled);
            if let Some(acc) = &item.accelerator {
                b = b.accelerator(acc);
            }
            builder.item(&b.build(app).map_err(menu_error)?)
        }
    })
}

fn build(app: &AppHandle, labels: &MenuLabels, items: &[MenuItemSpec]) -> AppResult<Menu<Wry>> {
    // Menu dell'app (macOS): Informazioni, Impostazioni… (⌘,), servizi, nascondi, Esci (⌘Q).
    let app_menu: Submenu<Wry> = SubmenuBuilder::new(app, &labels.app_name)
        .item(
            &MenuItemBuilder::with_id("help.about", &labels.about)
                .build(app)
                .map_err(menu_error)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id("nav.settings", &labels.settings)
                .accelerator("CmdOrCtrl+,")
                .build(app)
                .map_err(menu_error)?,
        )
        .separator()
        .item(&PredefinedMenuItem::services(app, None).map_err(menu_error)?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, None).map_err(menu_error)?)
        .item(&PredefinedMenuItem::hide_others(app, None).map_err(menu_error)?)
        .item(&PredefinedMenuItem::show_all(app, None).map_err(menu_error)?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some(&labels.quit)).map_err(menu_error)?)
        .build()
        .map_err(menu_error)?;
    let mut menu = MenuBuilder::new(app).item(&app_menu);
    for (title, rows) in layout(labels, items) {
        let sub = add_items(app, SubmenuBuilder::new(app, &title), &rows)?
            .build()
            .map_err(menu_error)?;
        menu = menu.item(&sub);
    }
    menu.build().map_err(menu_error)
}

/// Aggiorna il menu nativo (solo macOS): l'interfaccia lo richiama quando cambiano lingua o
/// disponibilità dei comandi.
#[tauri::command]
#[specta::specta]
pub fn set_app_menu(app: AppHandle, labels: MenuLabels, items: Vec<MenuItemSpec>) -> AppResult<()> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    let menu = build(&app, &labels, &items)?;
    app.set_menu(menu).map_err(menu_error)?;
    Ok(())
}

/// Inoltra all'interfaccia il comando scelto dal menu nativo.
pub fn forward(app: &AppHandle, id: &str) {
    let _ = app.emit(MENU_COMMAND, id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, menu: &str, section: u32, order: u32, submenu: Option<&str>) -> MenuItemSpec {
        MenuItemSpec {
            id: id.into(),
            label: id.into(),
            menu: menu.into(),
            section,
            order,
            submenu: submenu.map(Into::into),
            accelerator: None,
            enabled: true,
            checked: None,
        }
    }

    fn labels() -> MenuLabels {
        MenuLabels {
            app_name: "photo-cat".into(),
            about: "Informazioni su photo-cat".into(),
            settings: "Impostazioni…".into(),
            quit: "Esci da photo-cat".into(),
            menus: ["file", "edit", "view", "go", "tools", "help"]
                .iter()
                .map(|m| (m.to_string(), m.to_uppercase()))
                .collect(),
        }
    }

    #[test]
    fn voci_ordinate_per_menu_e_sezione_con_separatori_e_sottomenu() {
        let items = vec![
            item("view.fullscreen", "view", 3, 3, None),
            item("file.export", "file", 0, 1, None),
            item("file.import", "file", 0, 0, None),
            item("file.quit", "file", 3, 0, None),
            item("view.theme.dark", "view", 3, 1, Some("Tema")),
            item("view.theme.system", "view", 3, 0, Some("Tema")),
            item("nav.toggle", "view", 0, 0, None),
        ];
        let menus = layout(&labels(), &items);
        assert_eq!(
            menus.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(),
            ["FILE", "EDIT", "VIEW", "GO", "TOOLS", "HELP"]
        );
        let ids = |rows: &[Row]| -> Vec<String> {
            rows.iter()
                .map(|r| match r {
                    Row::Item(i) => i.id.clone(),
                    Row::Separator => "---".into(),
                    Row::Submenu(n, list) => format!(
                        "{n}[{}]",
                        list.iter()
                            .map(|i| i.id.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                })
                .collect()
        };
        assert_eq!(
            ids(&menus[0].1),
            ["file.import", "file.export", "---", "file.quit"]
        );
        assert_eq!(
            ids(&menus[2].1),
            [
                "nav.toggle",
                "---",
                "Tema[view.theme.system,view.theme.dark]",
                "view.fullscreen"
            ]
        );
        assert!(menus[1].1.is_empty());
    }
}
