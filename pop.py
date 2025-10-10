# --- Các process (hàm thuần) ---
def boil_water(ctx): 
    print("Đun nước sôi..."); ctx["water"] = "hot"; return ctx
def brew_coffee(ctx): 
    print("Pha cà phê..."); ctx["coffee"] = "brewed"; return ctx
def add_sugar(ctx): 
    print("Thêm đường..."); ctx["coffee"] += " + sugar"; return ctx
def add_milk(ctx): 
    print("Thêm sữa..."); ctx["coffee"] += " + milk"; return ctx
def taste_test(ctx): 
    print("Uống thử:", ctx["coffee"]); return ctx
def enjoy(ctx): 
    print("Thưởng thức:", ctx["coffee"]); return ctx

REGISTRY = {
    "boil_water": boil_water,
    "brew_coffee": brew_coffee,
    "add_sugar": add_sugar,
    "add_milk": add_milk,
    "taste_test": taste_test,
    "enjoy": enjoy,
}

# --- Engine rất gọn ---
def run_workflow(wf, ctx):
    for step in wf:
        if isinstance(step, str):      # step đơn
            ctx = REGISTRY[step](ctx)
        elif isinstance(step, list):   # step song song -> chạy tuần tự
            for s in step:
                ctx = REGISTRY[s](ctx)
    return ctx

# --- Demo ---
if __name__ == "__main__":
    choice = input("Chọn loại (black/milk): ").strip().lower()
    wf_black = ["boil_water", "brew_coffee", "add_sugar", "taste_test", "enjoy"]
    wf_milk  = ["boil_water", "brew_coffee", ["add_sugar","add_milk"], "taste_test", "enjoy"]
    workflows = {"black": wf_black, "milk": wf_milk}
    run_workflow(workflows[choice], {"coffee": ""})