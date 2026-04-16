window.waycal {
    background: transparent;
}
.waycal-root {
    background-color: {{ background }};
    border: 2px solid {{ accent }};
    border-radius: 0;
    padding: 14px 18px;
    color: {{ foreground }};
    font-family: "CaskaydiaMono Nerd Font", monospace;
    font-size: 13px;
    min-width: 260px;
}
.waycal-root.rounded {
    background-color: {{ background }};
    border: 2px solid transparent;
    border-radius: 16px;
}
.waycal-header {
    font-weight: bold;
    font-size: 15px;
    padding-bottom: 6px;
}
.waycal-weekday {
    color: {{ accent }};
    font-weight: bold;
    padding: 2px 6px;
}
.waycal-day {
    padding: 4px 7px;
    min-width: 18px;
}
.waycal-day.dim {
    opacity: 0.3;
}
.waycal-day.today {
    background-color: {{ accent }};
    color: {{ background }};
    border-radius: 0;
    font-weight: bold;
}
.waycal-root.rounded .waycal-day.today {
    border-radius: 8px;
}
.waycal-footer {
    color: {{ color8 }};
    font-size: 10px;
    padding-top: 8px;
    margin-top: 6px;
    border-top: 1px solid rgba(143, 188, 143, 0.18);
}
