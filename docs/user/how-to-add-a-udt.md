# Add a UDT type to a machine

## Purpose and reader

This guide shows how to define a new UDT type on a machine. It also shows how to create a new revision of an existing UDT.

The reader is a process engineer or an administrator. The result is the type tree of the new type or revision.

The editor shows Polish by default. The buttons **Polski** and **English** in the top bar change the language. This guide uses the English texts.

## Prerequisites

- You have the permission `UdtManage`. The roles Administrator and Process engineer have it. The role Operator does not (`api.md` section 3).
- The machine exists and is live. An archived machine has no link to its types.
- You know the PLC platform of the machine. The platform sets which primitive types your members can use (`data-model.md` section 7).
- Each struct that a member uses already exists on the machine as a revision, for example `ST_Zone@3`.

Without `UdtManage`, the list of types and the type tree still open. They show no link to the form (`api.md` section 2.3).

## Example

The procedure creates the type `ST_Zone` with three members:

| Member name | Type expression | Meaning |
|---|---|---|
| `temp_sp` | `REAL` | Temperature setpoint |
| `retry_count` | `INT` | Number of retries |
| `flags` | `ARRAY[1..4] OF BOOL` | Four flags |

All three type expressions exist on every platform (`data-model.md` section 7). The grammar of a type expression is in `domain-model.md` section 2.1.

## Add a new UDT

1. Select **Machines** in the navigation. Select your machine in the list.
2. On the page of the machine, select **Open the UDT types**.
3. Select **New UDT type**. The page **New UDT type** opens.
4. In **Type name**, enter `ST_Zone`.
5. In **Name of member 1**, enter `temp_sp`. In **Type of member 1**, enter `REAL`.
6. Select **Add a member**. The form keeps your input and shows one more empty row.
7. Fill **Name of member 2** and **Type of member 2** with `retry_count` and `INT`.
8. Select **Add a member** again. Fill the third row with `flags` and `ARRAY[1..4] OF BOOL`.
9. Select **Create type**.

The field **Type of member** is a text field. It proposes the primitives of the platform and the latest revision of each struct of the machine, as `<name>@<revision>`. You can select a proposal or type the expression (ADR-0037 (i)).

The server skips a row with an empty name and an empty type. A row with only one of the two fields gives an error.

## Result

The page **Type ST_Zone@1** opens. It shows these items:

- The list **Revisions** with a link for each revision.
- The section **Type tree** with one line for each member. A member with children shows as a node that you open or close.
- The links **Show the use of this revision** and **New revision**.

The type is immutable from now on. You change it only with a new revision (ADR-0006).

## Create a new revision of an existing UDT

Use this procedure to change the members of a type. You cannot edit a type that exists.

1. On the page of the machine, select **Open the UDT types**. Select the name of the type.
2. On the page of the type, select **New revision**. The page **New revision of ST_Zone** opens.
3. Change a name or a type expression.
4. Select **Add a member** for more rows.
5. Clear both fields of a row that you do not want.
6. Select **Create revision**.

The form holds the members of the latest revision. It has no field **Type name**, because the name stays the same.

The page **Type ST_Zone@2** opens. Its section **Supersedes** links to the revision before it.

The new revision does not change a recipe draft. A parameter keeps its old revision until you rebind it in the draft (ADR-0018). Select **Show the use of this revision** to see which parameters use a revision.

## Errors on this path

The server reads the name and every member row before it defines the type. It shows one error line under each bad input at once, and a summary at the top of the page (ADR-0041). Your input stays in the form.

| Code | What you see | What to do |
|---|---|---|
| `E-WEB-008` | A line under **Type name**, **Name of member**, or **Type of member** | Read the reason in the line. Fix the name or fill the empty field of the row |
| `E-PLT-013` | The type expression is not valid, with a reason | Use the grammar of `domain-model.md` section 2.1. A line under the field shows the reason |
| `E-PLT-006` | Member `x` uses a type that does not exist on the platform | Use a type of the platform of the machine, for example `INT` and not `LREAL` on `S7_300` |
| `E-PLT-021` | Two members have the same name | Give each member a different name. The line stands under the second row |
| `E-PLT-005` | The type contains itself | Delete the member that names the type. This includes a member of an earlier revision of the same type, for example `ST_Zone@1` (ADR-0038) |
| `E-PLT-015` | The type name exists on the machine | Open the existing type and select **New revision** |
| `E-PLT-016` | The type has no members | Fill at least one member row |
| `E-PLT-018` | The type is larger than the limit | Use fewer elements, fewer members, or fewer levels |
| `E-WEB-002` | Your role does not permit the operation | Ask an administrator for a role with `UdtManage` |
| `E-WEB-006` | The system is in read-only mode | Contact the administrator. The form saves nothing |

The reasons of `E-WEB-008` and the other lines are in the bundle `en`. The type name `new` is not valid, and neither is a primitive name. A type name and a member name start with a letter or an underscore. They have 1 to 64 letters, digits, and underscores.

The errors `E-PLT-005`, `E-PLT-006`, `E-PLT-015`, `E-PLT-016`, `E-PLT-018`, and `E-PLT-021` come from the command. The server returns one of them for each save. Fix it and select **Create type** or **Create revision** again.

The full list of codes is in `reliability.md` section 1.2.
