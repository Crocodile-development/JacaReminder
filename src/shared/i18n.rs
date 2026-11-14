// The language of the program. Every text of the interface is written in English in the code and
// goes through `t`, which gives the Portuguese version when that language is chosen.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
pub enum Language {
    #[default]
    English,
    Portuguese,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(language: Language) {
    CURRENT.store(language as u8, Ordering::Relaxed);
}

fn current() -> Language {
    if CURRENT.load(Ordering::Relaxed) == Language::Portuguese as u8 {
        Language::Portuguese
    } else {
        Language::English
    }
}

/// The text in the chosen language; a text with no translation stays as it is.
pub fn t(text: &str) -> &str {
    if current() == Language::Portuguese { portuguese(text).unwrap_or(text) } else { text }
}

pub fn weekday_names() -> [&'static str; 7] {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(|name| portuguese_or(name))
}

pub fn month_names() -> [&'static str; 12] {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        .map(|name| portuguese_or(name))
}

fn portuguese_or(text: &'static str) -> &'static str {
    if current() == Language::Portuguese { portuguese(text).unwrap_or(text) } else { text }
}

fn portuguese(text: &str) -> Option<&'static str> {
    Some(match text {
        // Navigation and common
        "Reminders" => "Lembretes",
        "History" => "Histórico",
        "Settings" => "Configurações",
        "Version" => "Versão",
        "Reminder" => "Lembrete",
        "Popup" => "Popup",
        "General" => "Geral",
        "Sound" => "Som",
        "Data" => "Dados",
        "Language" => "Idioma",
        "Save" => "Salvar",
        "Test" => "Testar",
        "Remove" => "Remover",
        "Edit" => "Editar",
        "Name" => "Nome",
        "Title" => "Título",
        "Message" => "Mensagem",
        "None" => "Nenhum",
        "and" => "e",
        // Home
        "No reminders yet" => "Nenhum lembrete ainda",
        "Press New reminder to create the first one." => "Clique em Novo lembrete para criar o primeiro.",
        "Nothing here" => "Nada aqui",
        "No reminder matches this filter." => "Nenhum lembrete corresponde a este filtro.",
        "All reminders are paused (Settings)" => "Todos os lembretes estão pausados (Configurações)",
        "{active} active of {total}" => "{active} ativos de {total}",
        "+ New reminder" => "+ Novo lembrete",
        "New reminder" => "Novo lembrete",
        "All" => "Todos",
        "Active" => "Ativos",
        "Paused" => "Pausados",
        "Next {time}" => "Próximo {time}",
        "No more firings" => "Sem mais disparos",
        "in {days}d {hours}h" => "em {days}d {hours}h",
        "in {hours}h {minutes}m" => "em {hours}h {minutes}m",
        "in {minutes}:{seconds}" => "em {minutes}:{seconds}",
        // History
        "Nothing has fired yet." => "Nada disparou ainda.",
        "Last 7 days" => "Últimos 7 dias",
        "Nothing in the last 7 days." => "Nada nos últimos 7 dias.",
        "(total {total})" => "(total {total})",
        "days from {day} to today" => "dias de {day} até hoje",
        "Latest" => "Mais recentes",
        // Editor
        "Delete this reminder?" => "Excluir este lembrete?",
        "\"{name}\" will be removed for good." => "\"{name}\" será removido para sempre.",
        "Delete" => "Excluir",
        "Cancel" => "Cancelar",
        "Delete reminder" => "Excluir lembrete",
        "Untitled reminder" => "Lembrete sem nome",
        "unsaved changes" => "alterações não salvas",
        // Schedule
        "Repeat" => "Repetir",
        "Times" => "Intervalo",
        "Times of the day" => "Horários do dia",
        "Years (none selected = every year)" => "Anos (nenhum = todos os anos)",
        "Add year" => "Adicionar ano",
        "Days and times" => "Dias e horários",
        "One time" => "Uma vez",
        "Date" => "Data",
        "Time" => "Hora",
        "Next: {time}" => "Próximo: {time}",
        "Next: never (nothing left in the future)" => "Próximo: nunca (nada no futuro)",
        "Every" => "A cada",
        "Quick" => "Atalhos",
        "Hours" => "Horas",
        "Only between" => "Somente entre",
        "+ Add time" => "+ Adicionar horário",
        "Days" => "Dias",
        "Every day" => "Todo dia",
        "Mon-Fri" => "Seg-Sex",
        "Weekend" => "Fim de semana",
        "Last day of the month" => "Último dia do mês",
        "Days of the week (none selected = every day)" => "Dias da semana (nenhum = todos os dias)",
        "More filters" => "Mais filtros",
        "Days of the month (none selected = every day)" => "Dias do mês (nenhum = todos os dias)",
        "Also the last day of each month" => "Também o último dia de cada mês",
        "Months (none selected = every month)" => "Meses (nenhum = todos os meses)",
        "Years" => "Anos",
        "2026, 2027 (empty = every year)" => "2026, 2027 (vazio = todos os anos)",
        "Starting on" => "A partir de",
        "Until" => "Até",
        "Mon" => "Seg",
        "Tue" => "Ter",
        "Wed" => "Qua",
        "Thu" => "Qui",
        "Fri" => "Sex",
        "Sat" => "Sáb",
        "Sun" => "Dom",
        "Feb" => "Fev",
        "Apr" => "Abr",
        "May" => "Mai",
        "Aug" => "Ago",
        "Sep" => "Set",
        "Oct" => "Out",
        "Dec" => "Dez",
        // Schedule summaries
        "Every {hours}h" => "A cada {hours}h",
        "Every {hours}h{minutes}" => "A cada {hours}h{minutes}",
        "Every {minutes} min" => "A cada {minutes} min",
        "{every}, {start} to {end}" => "{every}, das {start} às {end}",
        "{days} at {times}" => "{days} às {times}",
        "Once, {when}" => "Uma vez, {when}",
        "last" => "último",
        "day {days}" => "dia {days}",
        "month {months}" => "mês {months}",
        "year {years}" => "ano {years}",
        "{from} to {to}" => "{from} a {to}",
        "from {from}" => "de {from}",
        "until {to}" => "até {to}",
        // Tasks
        "Type" => "Tipo",
        "Alert" => "Alerta",
        "Task" => "Tarefa",
        "Shows the popup and closes by itself." => "Mostra o popup e fecha sozinho.",
        "Stays on the screen until you complete it. If you snooze, it comes back after the time below, until you complete it." => {
            "Fica na tela até você completar. Se você adiar, volta depois do tempo abaixo, até você completar."
        }
        "Snooze for" => "Adiar por",
        "Complete" => "Completar",
        "Snooze {minutes} min" => "Adiar {minutes} min",
        "Waiting for you" => "Esperando você",
        "Snoozed until {time}" => "Adiada até {time}",
        // Popup editor
        "Popup text" => "Texto do popup",
        "Look" => "Aparência",
        "Colors" => "Cores",
        "background" => "fundo",
        "text" => "texto",
        "bar" => "barra",
        "Position" => "Posição",
        "Size" => "Tamanho",
        "corners" => "cantos",
        "Behavior" => "Comportamento",
        "Stays for" => "Fica por",
        "(0 = until clicked)" => "(0 = até clicar)",
        "Progress bar" => "Barra de progresso",
        "Test popup" => "Testar popup",
        "Distance from edge" => "Distância da borda",
        "Horizontal" => "Horizontal",
        "Vertical" => "Vertical",
        "Rounded corners" => "Cantos curvos",
        "Corner radius" => "Curvatura",
        "Image" => "Imagem",
        "Image style" => "Estilo da imagem",
        "Icon" => "Ícone",
        "Background" => "Fundo",
        "Water drop" => "Gota d'água",
        "Windows beep" => "Bipe do Windows",
        "Your file" => "Seu arquivo",
        "Browse" => "Procurar",
        "Play" => "Tocar",
        "Stop" => "Parar",
        "This file could not be read as a sound" => "Não foi possível ler este arquivo como som",
        "The file has no sound" => "O arquivo não tem som",
        "Top right" => "Superior direito",
        "Top left" => "Superior esquerdo",
        "Bottom right" => "Inferior direito",
        "Bottom left" => "Inferior esquerdo",
        "Center" => "Centro",
        // Settings
        "App language" => "Idioma do app",
        "Changes the language of the whole program." => "Muda o idioma de todo o programa.",
        "Alert volume" => "Volume do alerta",
        "Applies to the water drop and to your own .wav files. The Windows beep follows the system volume." => {
            "Vale para a gota d'água e para os seus arquivos .wav. O bipe do Windows segue o volume do sistema."
        }
        "Pause all reminders" => "Pausar todos os lembretes",
        "No popup or sound while this is on. The schedules keep going, so reminders come back on their own times." => {
            "Sem popup nem som enquanto estiver ligado. Os horários continuam, então os lembretes voltam nos seus horários."
        }
        "Startup" => "Inicialização",
        "Start with Windows" => "Iniciar com o Windows",
        "Reminders keep working after you restart the computer." => {
            "Os lembretes continuam funcionando depois de reiniciar o computador."
        }
        "1 reminder fired so far." => "1 lembrete disparou até agora.",
        "{count} reminders fired so far." => "{count} lembretes dispararam até agora.",
        "Click again to clear" => "Clique de novo para limpar",
        "Clear history" => "Limpar histórico",
        "History " => "Histórico ",
        "Data folder" => "Pasta de dados",
        "Open folder" => "Abrir pasta",
        // Tray
        "Open dashboard" => "Abrir painel",
        "Quit" => "Sair",
        "Jaca Reminder - next {text}" => "Jaca Reminder - próximo {text}",
        _ => return None,
    })
}
