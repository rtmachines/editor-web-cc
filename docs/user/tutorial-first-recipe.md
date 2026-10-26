# First recipe in 15 minutes

In this lesson you create the recipe `R-100` on the machine `M1`. You add one parameter with a value and process limits. A colleague approves your draft. You then export the approved version as a JSON file.

## Before you start

You need these items:

- An account with the role **Process engineer**, for example the login `anowak`. You already changed your one-time password.
- The permissions `RecipeView`, `RecipeCreate`, `ParameterEdit`, and `VersionExport`. [reference-permissions.md](reference-permissions.md) lists the permissions of each role.
- The machine `M1` with the main unit `U1`. The PLC platform of `M1` is not **Siemens S7-300**.
- No recipe with the reference number `R-100`.
- A colleague with an account of the role **Administrator** or **Process engineer**. The colleague must not be you. The colleague approves your draft in steps 13 to 15.

Developers can get a practice environment. Set the variable `RECIPE_DEV_PASSWORD` to 12 to 128 characters. Then run `cargo run -p recipe-web --example dev_server` and open `http://127.0.0.1:8080/login`. The server seeds the logins `engineer` (Process engineer) and `admin` (Administrator) and no machine. Create `M1` first with the link **New machine** on the page **Machines**.

## Steps

The editor starts in Polish. This lesson shows the English texts. Step 2 switches the language.

### Step 1. Open the login page

Open the address of the editor in your browser. Add `/login` to the address.

You see the page **Log in** with the fields **Login** and **Password**.

### Step 2. Switch the language

Click **English** in the top bar.

You see the texts of the page in English. The buttons **Polski** and **English** stay in the top bar of every page.

### Step 3. Log in

Enter your login and your password. Click **Log in**.

You see the page **Recipes**. The top bar shows **Signed in as** and your login.

### Step 4. Open the machine

Click **Machines** in the top bar. Click `M1` in the column **Code**.

You see the page **Machine M1**. It shows the **PLC platform** and the section **Unit tree**. The tree shows the unit `U1`.

### Step 5. Look at the types

Click **Open the UDT types**.

You see the page **UDT types of machine M1**. This lesson uses the built-in type `INT`. The list of struct types can stay empty.

### Step 6. Open the form of a new recipe

Click **Recipes** in the top bar. Click **New recipe**.

You see the form **New recipe**.

### Step 7. Create the recipe

1. Select `M1` in the field **Machine**.
2. Enter `R-100` in the field **Reference number**.
3. Enter `Bottle 0.5 l` in the field **Name**.
4. Enter `First recipe from the tutorial` in the field **Change note**.
5. Click **Create recipe**.

You see the page **Version 1 of recipe R-100**. The badge **Draft** shows the state. The page shows your login as **Author** and the text **The version has no parameters.**

### Step 8. Open the form of a new parameter

Click **Add a parameter**.

You see the form **New parameter** below the table **Parameters**.

### Step 9. Describe the parameter

1. Select `U1` in the field **Unit**.
2. Enter `SPEED` in the field **Tag**.
3. Enter `Line speed` in the field **English name**.
4. Enter `Speed of the main line` in the field **English description**.
5. Enter `INT` in the field **Type**.
6. Keep `0` in the field **Order in the unit**.
7. Click **Show value fields**.

If the form shows the field **PLC address**, leave it empty.

You see the table **Values and process limits** with one row. The row has inputs in the columns **Value**, **Minimum**, and **Maximum**. The column **Maximum length** shows **not applicable**.

### Step 10. Set the value and the limits

1. Enter `120` in the column **Value**.
2. Enter `10` in the column **Minimum**.
3. Enter `250` in the column **Maximum**.
4. Click **Add parameter**.

You see the message **Parameter SPEED is added.** The table **Parameters** has a new row. The row shows the tag `SPEED`, the name `Line speed`, the type `INT`, and the unit `U1`.

### Step 11. Open the approval preview

Click **Approve version 1**.

You see the page **Approval of version 1 of recipe R-100**. An error notice shows the code `E-RCP-015`. The notice says that a different user must approve the draft. The page has no button **Approve version 1**, because you edited the draft.

### Step 12. Log out

Click **Log out** in the top bar.

You see the page **Log in**. Your colleague continues in the same browser.

### Step 13. Let your colleague open the draft

The colleague logs in with the own login and password. The colleague then clicks `R-100` in the list **Recipes**.

The colleague sees the page **Recipe R-100**. The table **Versions** shows version `1` with the badge **Draft**.

### Step 14. Let your colleague read the preview

The colleague clicks `1` in the column **Version**. The colleague then clicks **Approve version 1**.

The colleague sees the page **Approval of version 1 of recipe R-100**. The section **Summary of the changes** lists the added parameter `SPEED`. The heading **Approval** shows the text **Recipe R-100 has no approved version. Every parameter is new.**

### Step 15. Let your colleague approve the version

The colleague clicks the button **Approve version 1**.

The colleague sees the message **Version 1 is approved.** and the badge **Approved**. The line **Approver** shows the login of the colleague.

### Step 16. Log in again

The colleague clicks **Log out**. You log in with your own login and password. Click `R-100` in the list **Recipes**.

You see the page **Recipe R-100**. The table **Versions** shows version `1` with the badge **Approved**. The column **Approved at** shows a date.

### Step 17. Open the approved version

Click `1` in the column **Version**.

You see the page **Version 1 of recipe R-100** with the badge **Approved**. The page has no link **Add a parameter**. The button **Export version 1 as JSON** is on the page. [Why versions are immutable](explanation-why-versions-are-immutable.md) explains the missing link.

### Step 18. Export the version

Click **Export version 1 as JSON**.

Your browser saves the file `recipe_R-100_v1.json`. Open the file in a text editor. You see `"loadable": true` and a field `"content_sha256"` with 64 hexadecimal characters. The field `"version"` shows the number `1` and the status `"Approved"`. The parameter `SPEED` has the value `"120"`.

## Next steps

- [Edit a parameter in a draft](how-to-edit-a-parameter-in-a-draft.md)
- [Approve a version](how-to-approve-a-version.md)
- [Compare two versions](how-to-compare-two-versions.md)
- [Add a UDT](how-to-add-a-udt.md)
- [Export a version](how-to-export-a-version.md)
