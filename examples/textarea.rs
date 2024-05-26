use anyhow::anyhow;
use crossterm::cursor::{DisableBlinking, EnableBlinking, SetCursorStyle};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use log::debug;
use rat_event::ct_event;
use rat_input::event::{FocusKeys, HandleEvent, Outcome};
use rat_input::menuline::{MenuLine, MenuLineState, MenuOutcome};
use rat_input::statusline::{StatusLine, StatusLineState};
use rat_input::textarea::core::TextRange;
use rat_input::textarea::{TextArea, TextAreaState};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::{Frame, Terminal};
use std::fs;
use std::io::{stdout, Stdout};
use std::time::{Duration, SystemTime};

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        textarea: Default::default(),
        menu: Default::default(),
        status: Default::default(),
    };
    state.textarea.value.set_value(DATA1);

    state.textarea.add_style(TextRange::new((0, 0), (13, 0)), 0);
    state.textarea.add_style(TextRange::new((0, 1), (13, 1)), 0);
    state.textarea.add_style(TextRange::new((4, 3), (17, 3)), 0);
    state
        .textarea
        .add_style(TextRange::new((31, 44), (44, 44)), 0);

    // overlapping styles
    state
        .textarea
        .add_style(TextRange::new((30, 7), (42, 7)), 0);
    state
        .textarea
        .add_style(TextRange::new((37, 7), (41, 7)), 1);

    run_ui(&mut data, &mut state)
}

fn setup_logging() -> Result<(), anyhow::Error> {
    fs::remove_file("log.log")?;
    fern::Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("{}", message)))
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file("log.log")?)
        .apply()?;
    Ok(())
}

struct Data {}

struct State {
    pub(crate) textarea: TextAreaState,

    pub(crate) menu: MenuLineState,
    pub(crate) status: StatusLineState,
}

fn run_ui(data: &mut Data, state: &mut State) -> Result<(), anyhow::Error> {
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;
    stdout().execute(EnableBlinking)?;
    stdout().execute(SetCursorStyle::BlinkingBar)?;
    stdout().execute(EnableBracketedPaste)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    repaint_ui(&mut terminal, data, state)?;

    let r = 'l: loop {
        let o = match crossterm::event::poll(Duration::from_millis(10)) {
            Ok(true) => {
                let event = match crossterm::event::read() {
                    Ok(v) => v,
                    Err(e) => break 'l Err(anyhow!(e)),
                };
                match handle_event(event, data, state) {
                    Ok(v) => v,
                    Err(e) => break 'l Err(e),
                }
            }
            Ok(false) => continue,
            Err(e) => break 'l Err(anyhow!(e)),
        };

        match o {
            Outcome::Changed => {
                match repaint_ui(&mut terminal, data, state) {
                    Ok(_) => {}
                    Err(e) => break 'l Err(e),
                };
            }
            _ => {
                // noop
            }
        }
    };

    disable_raw_mode()?;
    stdout().execute(DisableBracketedPaste)?;
    stdout().execute(SetCursorStyle::DefaultUserShape)?;
    stdout().execute(DisableBlinking)?;
    stdout().execute(DisableMouseCapture)?;
    stdout().execute(LeaveAlternateScreen)?;

    r
}

fn repaint_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    data: &mut Data,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    terminal.hide_cursor()?;

    _ = terminal.draw(|frame| {
        repaint_tui(frame, data, state);
    });

    Ok(())
}

fn repaint_tui(frame: &mut Frame<'_>, data: &mut Data, state: &mut State) {
    let t0 = SystemTime::now();
    let area = frame.size();

    let l1 = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);

    repaint_input(frame, l1[0], data, state);

    let status1 = StatusLine::new()
        .layout([
            Constraint::Fill(1),
            Constraint::Length(27),
            Constraint::Length(17),
            Constraint::Length(17),
        ])
        .styles([
            Style::default().black().on_dark_gray(),
            Style::default().black().on_yellow(),
            Style::default().white().on_blue(),
            Style::default().white().on_light_blue(),
        ]);

    let el = t0.elapsed().unwrap_or(Duration::from_nanos(0));
    state
        .status
        .status(2, format!("Render {:?}", el).to_string());
    frame.render_stateful_widget(status1, l1[1], &mut state.status);
}

fn handle_event(
    event: crossterm::event::Event,
    data: &mut Data,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let t0 = SystemTime::now();

    let r = {
        match event {
            ct_event!(key press CONTROL-'q') => {
                return Err(anyhow!("quit"));
            }
            ct_event!(resized) => return Ok(Outcome::Changed),
            _ => {}
        }

        let r = handle_input(&event, data, state)?;

        r
    };

    let el = t0.elapsed().unwrap_or(Duration::from_nanos(0));
    state
        .status
        .status(3, format!("Handle {:?}", el).to_string());

    Ok(r)
}

fn repaint_input(frame: &mut Frame<'_>, area: Rect, _data: &mut Data, state: &mut State) {
    let l1 = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    let l2 = Layout::horizontal([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .split(l1[1]);

    let text = TextArea::new()
        .style(Style::default().black().on_dark_gray())
        .text_style([Style::new().red(), Style::new().underlined()]);
    frame.render_stateful_widget(text, l2[1], &mut state.textarea);

    let ccursor = state.textarea.selection();
    if let Some(cursor) = state.textarea.screen_cursor() {
        frame.set_cursor(cursor.x, cursor.y);
        state.status.status(
            1,
            format!(
                "{}|{} - {}|{} -> V{}|{}",
                ccursor.start().0,
                ccursor.start().1,
                ccursor.end().0,
                ccursor.end().1,
                cursor.x,
                cursor.y
            ),
        );
    } else {
        state.status.status(
            1,
            format!(
                "{}|{} - {}|{} -> None",
                ccursor.start().0,
                ccursor.start().1,
                ccursor.end().0,
                ccursor.end().1,
            ),
        );
    }

    let menu1 = MenuLine::new()
        .title("TextArea")
        .add("Text 1")
        .add("Text 2")
        .add("Text 3")
        .add("_Quit")
        .title_style(Style::default().black().on_yellow())
        .style(Style::default().black().on_dark_gray())
        .focused(true);
    frame.render_stateful_widget(menu1, l1[2], &mut state.menu);
}

fn handle_input(
    event: &crossterm::event::Event,
    _data: &mut Data,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let r = state.textarea.handle(event, FocusKeys);
    if r != Outcome::NotUsed {
        return Ok(r);
    }

    let r = HandleEvent::handle(&mut state.menu, event, FocusKeys);
    match r {
        MenuOutcome::Selected(v) => {
            state.status.status(0, format!("Selected {}", v));
        }
        MenuOutcome::Activated(v) => {
            state.status.status(0, format!("Activated {}", v));
            match v {
                3 => return Err(anyhow!("Quit")),
                _ => {}
            }
        }
        _ => {}
    };

    Ok(r.into())
}

static DATA1: &str = "Ridley Scott
Ridley Scott (2015)

Sir Ridley Scott GBE[1] (* 30. November 1937 in South Shields, England) ist ein
britischer Filmregisseur und Filmproduzent. Er gilt heute als einer der
renommiertesten und einflussreichsten Regisseure und hat die Erzählweisen
mehrerer Filmgenres geprägt. Er wurde nie mit einem Oscar ausgezeichnet.
Seine bekanntesten Filme sind Alien (1979), Blade Runner (1982), Thelma & Louise
(1991), Gladiator (2000), Black Hawk Down (2001) und Der Marsianer (2015).

Scott ist Eigentümer der 1995 gegründeten Filmproduktionsfirma Scott Free Productions.
Inhaltsverzeichnis

    1 Leben
    2 Werk
    3 Filmografie (Auswahl)
    4 Auszeichnungen (Auswahl)
    5 Literatur
    6 Weblinks
    7 Einzelnachweise

Leben

Scott wurde als Sohn eines Berufssoldaten geboren. Sein Vater, den er selten
zu sehen bekam, diente bei den Royal Engineers (Kampfunterstützungstruppen der
britischen Armee) in hoher Position als Ingenieur und Transportkontrolleur.
Nach Aufenthalten in Cumbria, Wales und Deutschland (dort zwischen 1947 und
1952 in Hamburg) ließ sich die Familie in Stockton-on-Tees im Norden Englands
nieder (die industriell geprägte Landschaft inspirierte später Szenen in Blade
Runner). Zum Ende seiner Kindheit und Jugend hatte er eigenen Angaben zufolge
wegen der vielen Umzüge 10 Schulen besucht.[2]

Scott erlernte 1954 bis 1958 Grafikdesign und Malerei am West Hartlepool College
of Art und erlangte das Diplom mit Auszeichnung. Er studierte daraufhin
Grafikdesign (M.A., 1960 bis 1962) am Royal College of Art in London, wo
David Hockney einer seiner Mitstudenten war. Er schloss 1963 mit Auszeichnung
ab. Scott erhielt ein einjähriges Reisestipendium in die USA und wurde bei Time Life
beschäftigt, wo er mit den Dokumentaristen Richard Leacock und D. A. Pennebaker
arbeitete. Nach seiner Rückkehr nahm er 1965 eine Lehrstelle bei der BBC als
Szenenbildner an. Diese Position führte ihn zur Mitarbeit an beliebten
Fernsehproduktionen wie der Polizei-Serie Z-Cars oder der Science-Fiction-Serie
Out of the Unknown. Nach kurzer Zeit wurde er ins Trainingsprogramm für Regisseure
aufgenommen und inszenierte einige Episoden selbst.

1968 verließ Scott die BBC, um Ridley Scott Associates (RSA) zu gründen. An dem
Projekt arbeiteten neben seinem Bruder Tony Regisseure wie Alan Parker, Hugh Hudson
und Hugh Johnson mit. RSA wurde zu einem der erfolgreichsten Werbefilm-Häuser in
Europa, in dessen Auftrag Scott für über 2000 Werbespots verantwortlich zeichnet;
viele davon wurden auf den Festspielen von Cannes und Venedig ausgezeichnet.

Scott gilt in der Branche als ökonomischer Regisseur, da er in der Regel mit einem
Drittel der Drehtage seiner Kollegen auskommt. Eigenen Worten zufolge verdankt
er dies seiner Vergangenheit als Werbe- und Videospotregisseur sowie der Tatsache,
dass er manche Szenen mit bis zu 15 Kameras gleichzeitig drehe.[3] Seit dem Jahr 2000,
als sie in Gladiator eine Nebenrolle spielte, ist Scott mit der costa-ricanischen
Schauspielerin Giannina Facio, Tochter des Diplomaten und Politikers Gonzalo Facio
(1918–2018), liiert. Im Juni 2015 heiratete das Paar.[4]

Im Jahr 2003 wurde Scott von der britischen Königin aufgrund seiner Verdienste um
die Kunst zum Ritter geschlagen, am 8. Mai 2024 ernannte Thronfolger Prinz William
ihn zum Knight Grand Cross of the Order of the British Empire. Scott ist damit
Träger des höchsten britischen Verdienstordens.

Sein jüngerer Bruder ist der Regisseur und Filmproduzent Tony Scott, der sich 2012
das Leben nahm. Seine Söhne Luke und Jake und seine Tochter Jordan sind ebenfalls
im Filmgeschäft tätig.

Scott lebt in Los Angeles, besitzt aber seit etwa Anfang der 90er Jahre ein Haus
in Südfrankreich.[2]
Werk

Scotts Markenzeichen ist ein ausgeprägt ästhetischer und malerischer visueller Stil,
der sich durch seine jahrelange Erfahrung als Production Designer und Regisseur
von Werbespots entwickelt hat. Zusammen mit seinem Bruder Tony betrieb er ab
1968 die Produktionsfirma für Werbefilme Ridley Scott Associates (RSA).

Scotts erster Themenfilm Die Duellisten (1977) war zwar kommerziell kein großer
Erfolg, fand aber bei der Kritik genug Beachtung, um Scott die Realisierung des
Science-Fiction-Films Alien – Das unheimliche Wesen aus einer fremden Welt (1979)
zu ermöglichen. Sein nächster Film Blade Runner (1982), basierend auf dem Roman
Träumen Androiden von elektrischen Schafen? von Philip K. Dick, spielt in einem
düster-futuristischen Los Angeles. Das Werk war visuell derart beeindruckend, dass
es für eine ganze Generation Cyberpunk-Literatur, -Musik und -Kunst als Inspiration
diente. In der Folge drehte Scott Legende (1985), Der Mann im Hintergrund (1987)
und Black Rain (1989), die alle nicht an die Bedeutung und den Erfolg der vorigen
Werke anknüpfen konnten. Legende setzte sich jedoch im Lauf der Zeit als Fantasy-Kultfilm
durch und wurde 2002 mit einem restaurierten Director’s Cut ergänzt.

Die von der Kritik stetig vorgebrachte Beschuldigung, visuellen Stil vor Inhalt und
Charakterzeichnung zu stellen, wurde mit Thelma & Louise (1991) entkräftet. Neben
guten Kritiken erhielt Scott seine erste Oscar-Nominierung für die beste Regie.
Danach folgten mit dem Kolumbus-Film 1492 – Die Eroberung des Paradieses (1992),
White Squall – Reißende Strömung (1996) und Die Akte Jane (1997) erneut Filme, die
künstlerisch und kommerziell durchfielen. Insbesondere der Militärfilm Die Akte Jane,
in dem Demi Moore eine Frau spielt, die als erste Mitglied bei den Navy Seals
werden will, wurde wegen einer nach Ansicht vieler Kritiker undifferenzierten
Pro-Militär-Haltung angegriffen. Mit Gladiator feierte Scott 2000 ein triumphales
Comeback. Der Film war beim Publikum sehr erfolgreich und gewann neben dem Oscar
für den besten Film im Jahr 2000 auch den Golden Globe 2001. Die Regie-Leistung
wurde ebenfalls nominiert, den Preis erhielt Scott jedoch nicht. Eine weitere
Oscar-Nominierung erhielt er für den kontroversen Kriegsfilm Black Hawk Down
(2001), der einen verunglückten US-amerikanischen Militäreinsatz in Somalia
thematisiert und in eindrucksvolle Bilder umsetzt. Black Hawk Down prägte die
neuere Action-Darstellung und verhalf der dokumentaristischen Kameraführung zum
Durchbruch in der Filmkunst.

Scott übernahm die Regie bei dem Film Hannibal (2001), der Fortsetzung zu Das
Schweigen der Lämmer (1991). 2005/2006 folgte in zwei Versionen der Film Königreich
der Himmel. 2006 erschien Ein gutes Jahr nach dem Roman Ein guter Jahrgang seines
Landsmannes Peter Mayle. Er handelt von einem Bankmanager, der von seinem Onkel
ein Weingut in der Provence erbt und daraufhin beschließt, sein Leben umzukrempeln.
Die Hauptrolle spielt der neuseeländische Schauspieler Russell Crowe. Gemeinsam
mit seinem Bruder Tony produzierte Scott für den amerikanischen Kabelsender TNT
die Miniserie The Company – Im Auftrag der CIA, die im August 2007 ausgestrahlt
wurde. The Company erzählt die Geschichte dreier Yale-Absolventen, die in der
Nachkriegszeit auf Seiten der CIA bzw. des KGB in den Kalten Krieg verwickelt
werden. In den Hauptrollen sind u. a. Chris O’Donnell, Michael Keaton und Alfred
Molina zu sehen.

Im Oktober 2008 bestätigte Scott, dass er 25 Jahre warten musste, bis die Rechte an
dem Buch Der Ewige Krieg von Joe Haldeman für eine Verfilmung zur Verfügung standen.
[5] Scott plane, dieses Buch in 3D zu verfilmen.[6]

Für den US-Fernsehsender CBS produzierte Scott seit 2009 die Serie Good Wife.
Die Ausstrahlung begann in den USA im September 2009, in Deutschland bei ProSieben
Ende März 2010. Auch hier arbeitete er mit seinem Bruder Tony zusammen. Mit der
2009 abgedrehten Produktion Robin Hood legte Scott erneut einen Historienfilm
vor. Mit seinem 22. Spielfilm, realisiert nach einem Drehbuch von Brian Helgeland
mit Russell Crowe in der Titelrolle, wurden am 12. Mai 2010 die 63. Filmfestspiele
von Cannes eröffnet.[7]

Scott arbeitete 2009 an der ersten Verfilmung von Aldous Huxleys Roman Schöne neue
Welt für das Kino. Der Film sollte von ihm und Leonardo DiCaprio produziert werden,
Drehbuchautor sollte Farhad Safinia sein. Scott sollte voraussichtlich auch Regie
führen, der Film wurde jedoch bis heute nicht realisiert.[8] Der Film Prometheus
war ursprünglich als Prequel zu Scotts erstem großen Erfolg Alien geplant. Das
Drehbuch stammt von Jon Spaihts; Damon Lindelof überarbeitete das Drehbuch für
20th Century Fox. In den USA erfolgte der Kinostart am 8. Juni 2012. 2017
folgte die Fortsetzung Alien: Covenant. Im selben Jahr verfilmte Scott mit
Alles Geld der Welt den Entführungsfall um John Paul Getty III. Im Zuge des
Skandals um Kevin Spacey, der ab Ende Oktober 2017 mit Vorwürfen der sexuellen
Belästigung konfrontiert wurde, entschloss sich das Filmteam und Sony Pictures,
alle Szenen mit Spacey aus dem Film zu schneiden. Scott musste diese Szenen
kurzfristig mit Christopher Plummer nachdrehen. ";
