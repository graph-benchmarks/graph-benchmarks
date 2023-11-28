import toml
import shutil

config=toml.load("build.config.toml")
workspace=toml.load("Cargo.toml")
provider_base=toml.load("providers/base-provider/Cargo.toml")
driver_base=toml.load("drivers/base-driver/Cargo.toml")

try:
    previous_config=toml.load("previous.build.config.toml")
    for provider in previous_config["providers"]:
        workspace["workspace"]["members"].remove("providers/" + provider)
        del provider_base["dependencies"][provider]

    for driver in previous_config["drivers"]:
        workspace["workspace"]["members"].remove("drivers/" + driver + "/" + driver + "-config")
        del driver_base["dependencies"][driver + "-config"]
except:
    provider_base

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

shutil.copyfile("build.config.toml", "previous.build.config.toml")

drivers_str = "\n".join(config["drivers"])
f = open(".build-drivers", "tw")
f.write(drivers_str)