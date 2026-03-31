import streamlit as st
import pandas as pd
import plotly.express as px
import glob
import os
import re

st.set_page_config(page_title="PACE 2026 MAF Dashboard", layout="wide")

st.title("🌲 PACE 2026 MAF Solver Dashboard")
st.markdown("""
This dashboard visualizes the performance of the Rust and Python solvers for the Maximum Agreement Forest (MAF) problem.
""")

def load_summary():
    if not os.path.exists("results/summary.md"):
        return None
    
    with open("results/summary.md", "r") as f:
        content = f.read()
    
    # Split by runs
    runs = content.split("### Run ")
    data = []
    for run in runs[1:]:
        lines = run.strip().split("\n")
        timestamp = lines[0].strip()
        # Find table rows
        for line in lines:
            if line.startswith("|") and "Instance" not in line and "---" not in line:
                parts = [p.strip() for p in line.split("|")]
                if len(parts) >= 6:
                    data.append({
                        "Run": timestamp,
                        "Instance": parts[2],
                        "30s": parts[3],
                        "60s": parts[4],
                        "120s": parts[5]
                    })
    return pd.DataFrame(data)

def parse_benchmark_file(file_path):
    with open(file_path, "r") as f:
        lines = f.readlines()
    
    data = []
    # Skip header lines
    for line in lines[4:]:
        parts = [p.strip() for p in line.split("|")]
        if len(parts) == 4:
            data.append({
                "Instance": parts[0],
                "30s": parts[1],
                "60s": parts[2],
                "120s": parts[3]
            })
    return pd.DataFrame(data)

df_all = load_summary()

if df_all is not None and not df_all.empty:
    st.sidebar.header("Filters")
    instances = df_all["Instance"].unique()
    selected_instances = st.sidebar.multiselect("Select Instances", instances, default=instances[:3])
    
    col1, col2 = st.columns(2)
    
    with col1:
        st.subheader("Latest Results Summary")
        latest_run = df_all["Run"].iloc[-1]
        st.write(f"Showing results for run: **{latest_run}**")
        latest_df = df_all[df_all["Run"] == latest_run]
        st.dataframe(latest_df, use_container_width=True)

    with col2:
        st.subheader("Performance Trend (Anytime Quality)")
        # Melt dataframe for plotting
        plot_df = latest_df[latest_df["Instance"].isin(selected_instances)].melt(
            id_vars=["Instance"], 
            value_vars=["30s", "60s", "120s"],
            var_name="Time Limit", 
            value_name="Components"
        )
        # Convert Components to numeric, handle ERR/TO
        plot_df["Components"] = pd.to_numeric(plot_df["Components"], errors='coerce')
        plot_df = plot_df.dropna()
        
        if not plot_df.empty:
            fig = px.line(plot_df, x="Time Limit", y="Components", color="Instance", markers=True,
                         title="Components vs. Time Budget")
            st.plotly_chart(fig, use_container_width=True)
        else:
            st.info("No numeric data available for the selected instances.")

    st.divider()
    st.subheader("Historical Comparison")
    hist_df = df_all[df_all["Instance"].isin(selected_instances)].copy()
    hist_df["120s"] = pd.to_numeric(hist_df["120s"], errors='coerce')
    
    fig_hist = px.bar(hist_df, x="Run", y="120s", color="Instance", barmode="group",
                     title="Quality improvement across different solver versions (120s results)")
    st.plotly_chart(fig_hist, use_container_width=True)

else:
    st.warning("No benchmark results found in `results/summary.md`. Run `python3 compare_times.py` first.")

st.sidebar.markdown("---")
st.sidebar.info("To run a new benchmark, use:\n`python3 compare_times.py` from the root directory.")
