// ==== FUN_004793f4 @ 004793f4 ====

void FUN_004793f4(longlong *param_1,undefined8 *param_2,int param_3,undefined4 param_4)

{
  int *piVar1;
  uint *puVar2;
  longlong *plVar3;
  undefined1 uVar4;
  byte *pbVar5;
  uint uVar6;
  byte *pbVar7;
  undefined1 *puVar8;
  uint uVar9;
  uint uVar10;
  uint uVar11;
  
  FUN_004774dc(param_2);
  FUN_00478690(param_3,param_4);
LAB_0047941c:
  pbVar7 = DAT_005531e0;
  if (*DAT_005531e0 == 0) {
    FUN_004784a0((longlong)DAT_005531e0);
    goto LAB_0047951c;
  }
  FUN_00479104(DAT_005531e0);
  uVar6 = DAT_00553f90;
LAB_004794ef:
  DAT_00553f90 = uVar6 + DAT_00553f98 * DAT_00553f88;
  DAT_00553f88 = DAT_00553f88 * (DAT_00553f9c - DAT_00553f98);
LAB_0047951c:
  uVar10 = DAT_00553f90;
  uVar6 = DAT_00553f88;
  puVar8 = DAT_00552fc8;
  if (DAT_00552fc8 == (undefined1 *)0x0) {
    do {
      uVar6 = DAT_00553f90;
      if (0xffffff < (DAT_00553f88 + DAT_00553f90 ^ DAT_00553f90)) {
        if (0x7fff < DAT_00553f88) goto LAB_004794cf;
        DAT_00553f88 = -DAT_00553f90 & 0x7fff;
      }
      uVar10 = DAT_00553f88;
      piVar1 = (int *)(param_2 + 2);
      *piVar1 = *piVar1 + -1;
      if (*piVar1 < 0) {
        uVar9 = (**(code **)*param_2)(param_2);
      }
      else {
        uVar9 = (uint)*(byte *)param_2[3];
        param_2[3] = (byte *)param_2[3] + 1;
        DAT_00553f90 = uVar6;
        DAT_00553f88 = uVar10;
      }
      DAT_00553f88 = DAT_00553f88 << 8;
      DAT_00553f8c = DAT_00553f8c << 8 | uVar9;
      DAT_00553f90 = DAT_00553f90 << 8;
    } while( true );
  }
  uVar4 = *DAT_00552fc8;
  plVar3 = param_1 + 2;
  *(int *)plVar3 = (int)*plVar3 + -1;
  if ((int)*plVar3 < 0) {
    (**(code **)(*param_1 + 8))(param_1);
    puVar8 = DAT_00552fc8;
    uVar6 = DAT_00553f88;
    uVar10 = DAT_00553f90;
  }
  else {
    *(undefined1 *)param_1[3] = uVar4;
    param_1[3] = param_1[3] + 1;
  }
  if (((DAT_00552fbc != 0) ||
      (uVar9 = DAT_00553f88, uVar11 = DAT_00553f90,
      pbVar5 = (byte *)(ulonglong)*(uint *)(puVar8 + 2),
      (byte *)(ulonglong)*(uint *)(puVar8 + 2) < DAT_00553ec0)) &&
     (FUN_00478c74(pbVar7), uVar9 = uVar6, uVar11 = uVar10, pbVar5 = DAT_005531e0,
     DAT_00552ea1 == '\0')) {
    DAT_00552ea1 = '\x01';
    FUN_0050daf0((ulonglong *)&DAT_00552eb0,0,0x100);
    DAT_00552ea0 = DAT_00552ea0 + '\x01';
    pbVar5 = DAT_005531e0;
    if (DAT_00552ea0 == '\0') {
      FUN_0046ea50();
      uVar9 = DAT_00553f88;
      uVar11 = DAT_00553f90;
      pbVar5 = DAT_005531e0;
    }
  }
  do {
    DAT_005531e0 = pbVar5;
    if (0xffffff < (uVar9 + uVar11 ^ uVar11)) {
      if (0x7fff < uVar9) goto LAB_0047941c;
      uVar9 = -uVar11 & 0x7fff;
      DAT_00553f88 = uVar9;
    }
    piVar1 = (int *)(param_2 + 2);
    *piVar1 = *piVar1 + -1;
    if (*piVar1 < 0) {
      uVar6 = (**(code **)*param_2)(param_2);
    }
    else {
      uVar6 = (uint)*(byte *)param_2[3];
      param_2[3] = (byte *)param_2[3] + 1;
      DAT_00553f88 = uVar9;
      DAT_00553f90 = uVar11;
    }
    DAT_00553f88 = DAT_00553f88 << 8;
    DAT_00553f8c = DAT_00553f8c << 8 | uVar6;
    DAT_00553f90 = DAT_00553f90 << 8;
    uVar9 = DAT_00553f88;
    uVar11 = DAT_00553f90;
    pbVar5 = DAT_005531e0;
  } while( true );
LAB_004794cf:
  do {
    puVar2 = (uint *)(pbVar7 + 8);
    pbVar7 = (byte *)(ulonglong)*puVar2;
    if (*puVar2 == 0) {
      FUN_0046ea50();
      return;
    }
    DAT_00552fbc = DAT_00552fbc + 1;
  } while (*pbVar7 == DAT_00552ea3);
  FUN_00479290(pbVar7);
  goto LAB_004794ef;
}


