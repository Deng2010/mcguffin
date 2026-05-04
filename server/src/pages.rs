use axum::{
    extract::State,
    response::{Html, Redirect},
};
use crate::state::AppState;

// ============== Login Page ==============

pub async fn login_page() -> Html<String> {
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Login - McGuffin</title>
        <style>
            body { font-family: system-ui, sans-serif; display: flex; justify-content: center; align-items: center; min-height: 100vh; margin: 0; background: #f3f4f6; }
            .card { background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); text-align: center; }
            h1 { margin: 0 0 0.5rem; color: #1f2937; }
            p { color: #6b7280; margin-bottom: 1.5rem; }
            .btn { display: inline-block; padding: 0.75rem 1.5rem; background: #111827; color: white; text-decoration: none; border-radius: 4px; font-weight: 500; }
            .btn:hover { background: #374151; }
        </style>
    </head>
    <body>
        <div class="card">
            <h1>McGuffin</h1>
            <p>算法竞赛出题团队工具</p>
            <a href="/api/oauth/authorize" class="btn">通过 CP OAuth 登录</a>
            <div style="margin-top:1.5rem;padding-top:1.5rem;border-top:1px solid #e5e7eb;font-size:0.875rem;color:#6b7280;">
                <p><a href="/portfolio" style="color:#111827;">游客访问</a></p>
            </div>
        </div>
    </body>
    </html>
    "#;
    Html(html.to_string())
}

// ============== Portfolio Page ==============

pub async fn portfolio_page(State(state): State<AppState>) -> Html<String> {
    let problems = state.problems.read().await;
    let problem_items: Vec<String> = problems
        .values()
        .filter(|p| p.status == "published")
        .map(|p| {
            format!(
                r#"<div class="problem-card">
                    <h3>{}</h3>
                    <p class="meta">作者：{} | 赛事：{} | 公开于：{}</p>
                    <span class="badge badge-{}">{}</span>
                </div>"#,
                p.title, p.author_name, p.contest, 
                p.public_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
                p.difficulty.to_lowercase(),
                p.difficulty
            )
        })
        .collect();

    let html = format!(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>成果展示 - McGuffin</title>
        <style>
            body {{ font-family: system-ui, sans-serif; margin: 0; background: #f3f4f6; }}
            .container {{ max-width: 800px; margin: 0 auto; padding: 2rem; }}
            h1 {{ color: #1f2937; }}
            .problem-card {{ background: white; padding: 1.5rem; margin-bottom: 1rem; border-radius: 8px; border: 1px solid #e5e7eb; }}
            .problem-card h3 {{ margin: 0 0 0.5rem; color: #1f2937; }}
            .meta {{ color: #6b7280; font-size: 0.875rem; margin: 0 0 1rem; }}
            .badge {{ display: inline-block; padding: 0.25rem 0.75rem; border-radius: 9999px; font-size: 0.875rem; font-weight: 500; }}
            .badge-easy {{ background: #d1d5db; color: #374151; }}
            .badge-medium {{ background: #6b7280; color: white; }}
            .badge-hard {{ background: #1f2937; color: white; }}
            .nav {{ background: white; padding: 1rem 2rem; border-bottom: 1px solid #e5e7eb; display: flex; gap: 1.5rem; }}
            .nav a {{ color: #4b5563; text-decoration: none; }}
            .nav a:hover {{ color: #1f2937; }}
            .btn {{ display: inline-block; padding: 0.5rem 1rem; background: #111827; color: white; text-decoration: none; border-radius: 4px; font-size: 0.875rem; }}
            .empty {{ text-align: center; color: #9ca3af; padding: 3rem; }}
        </style>
    </head>
    <body>
        <nav class="nav">
            <a href="/"><strong>McGuffin</strong></a>
            <a href="/portfolio">成果展示</a>
            <a href="/login">登录</a>
        </nav>
        <div class="container">
            <h1>成果展示</h1>
            {}
        </div>
    </body>
    </html>
    "#,
        if problem_items.is_empty() {
            "<div class='empty'>暂无成果展示</div>".to_string()
        } else {
            problem_items.join("\n")
        }
    );

    Html(html)
}

// ============== Root Redirect (legacy) ==============

pub async fn root() -> Redirect {
    Redirect::to("/portfolio")
}
