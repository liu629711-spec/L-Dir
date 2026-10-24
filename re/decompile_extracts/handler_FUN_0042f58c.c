// ==== FUN_0042f58c @ 0042f58c ====

undefined8
FUN_0042f58c(longlong param_1,undefined8 param_2,int param_3,HMENU param_4,undefined8 param_5,
            undefined8 *param_6,int param_7)

{
  ushort uVar1;
  bool bVar2;
  uint uVar3;
  undefined8 uVar4;
  ushort *puVar5;
  ushort *puVar6;
  
  if (param_7 == 0) {
    if (param_3 == 0x117) {
      if ((param_4 == (HMENU)0x0) ||
         (puVar5 = *(ushort **)(param_1 + 0x18), puVar5 == (ushort *)0x0)) {
        uVar4 = 1;
      }
      else {
        puVar6 = *(ushort **)(param_1 + 0x10);
        uVar1 = *puVar6;
        while (uVar1 != 0xffff) {
          if ((puVar6[1] & 1) != 0) {
            FUN_0042f48c((uint)uVar1,puVar5,param_4);
          }
          puVar6 = puVar6 + 2;
          puVar5 = puVar5 + 8;
          uVar1 = *puVar6;
        }
        uVar4 = 0;
      }
      *param_6 = uVar4;
    }
    else if (param_3 == 0x111) {
      bVar2 = false;
      if ((*(char *)(param_1 + 0x22) != '\0') &&
         (bVar2 = false, (short)((ulonglong)param_4 >> 0x10) == 1)) {
        puVar5 = *(ushort **)(param_1 + 0x18);
        puVar6 = *(ushort **)(param_1 + 0x10);
        if (puVar5 != (ushort *)0x0) {
          for (; *puVar6 != 0xffff; puVar6 = puVar6 + 2) {
            if (((uint)param_4 & 0xffff) == (uint)*puVar6) {
              uVar3 = (uint)*puVar5;
              goto LAB_0042f63d;
            }
            puVar5 = puVar5 + 8;
          }
        }
        uVar3 = 0;
LAB_0042f63d:
        bVar2 = false;
        if ((uVar3 >> 8 & 1) != 0) {
          bVar2 = true;
        }
      }
      *param_6 = 0;
      if (bVar2) {
        return 1;
      }
    }
  }
  return 0;
}


