import toml
import re

config=toml.load("build.config.toml")
workspace=toml.load("Cargo.toml")
provider_base=toml.load("providers/base-provider/Cargo.toml")
driver_base=toml.load("drivers/base-driver/Cargo.toml")

for member in workspace["workspace"]["members"]:
    if member != "providers/base-provider" and member.startswith("providers/"):
        workspace["workspace"]["members"].remove(member)
    elif member != "drivers/base-driver" and member.startswith("drivers/"):
        workspace["workspace"]["members"].remove(member)
print(workspace["workspace"])

remove_entries = []
for dep in provider_base["dependencies"]:
    if "path" in provider_base["dependencies"][dep] and re.search("^[.]{2}/\w", provider_base["dependencies"][dep]["path"]) is not None:
        remove_entries.append(dep)
for item in remove_entries:
    del provider_base["dependencies"][item]

remove_entries = []
for dep in driver_base["dependencies"]:
    if "path" in driver_base["dependencies"][dep] and re.search("^[.]{2}/\w", driver_base["dependencies"][dep]["path"]) is not None:
        remove_entries.append(dep)
for item in remove_entries:
    del driver_base["dependencies"][item]

for provider in config["providers"]:
    workspace["workspace"]["members"].append("providers/" + provider)
    provider_base["dependencies"][provider] = {"path": "../" + provider}

for driver in config["drivers"]:
    workspace["workspace"]["members"].append("drivers/" + driver + "/" + driver + "-config")
    driver_base["dependencies"][driver + "-config"] = {"path": "../" + driver + "/" + driver + "-config"}

f = open("Cargo.toml", "w")
toml.dump(workspace, f)

f = open("providers/base-provider/Cargo.toml", "w")
toml.dump(provider_base, f)

f = open("drivers/base-driver/Cargo.toml", "w")
toml.dump(driver_base, f)

drivers_str = "\n".join(config["drivers"])
f = open(".build-drivers", "tw")
f.write(drivers_str)